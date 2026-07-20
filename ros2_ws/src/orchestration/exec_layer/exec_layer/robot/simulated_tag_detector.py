from __future__ import annotations

import json
import math
import os
import random
from typing import Optional

import rclpy
from rclpy.node import Node
from builtin_interfaces.msg import Time as TimeMsg

from apriltag_interfaces.msg import AprilTagDetection, AprilTagDetections


class SimulatedAprilTagDetector(Node):
    """Geometric AprilTag simulation for MuJoCo.

    Does NOT render images. Computes tag detections by projecting
    tag world coordinates into the camera frame using the robot pose
    obtained from unitree DDS rt/sportmodestate.

    Publishes to /perception/apriltag_detections at ~10Hz.
    """

    def __init__(
        self,
        maps_path: str,
        publish_rate: float = 10.0,
        fov_h_deg: float = 45.0,
        fov_v_deg: float = 35.0,
        min_distance: float = 0.3,
        max_distance: float = 5.0,
        noise_offset: float = 0.02,
        noise_distance: float = 0.01,
        noise_angle: float = 0.5,
    ) -> None:
        super().__init__("simulated_apriltag_detector")

        self._maps_path = maps_path
        self._fov_h = math.radians(fov_h_deg)
        self._fov_v = math.radians(fov_v_deg)
        self._min_dist = min_distance
        self._max_dist = max_distance
        self._noise_offset = noise_offset
        self._noise_distance = noise_distance
        self._noise_angle = math.radians(noise_angle)

        # Robot pose in simulation (populated via DDS bridge)
        self._robot_x = 0.0
        self._robot_y = 0.0
        self._robot_yaw = 0.0

        # Load tag map
        self._tags: dict[str, dict] = {}
        self._load_map()

        # Publisher
        self._pub = self.create_publisher(AprilTagDetections, "/perception/apriltag_detections", 10)

        # Timer for periodic detection
        period = 1.0 / publish_rate if publish_rate > 0 else 0.1
        self._timer = self.create_timer(period, self._detect_tags)

        self.get_logger().info(
            f"SimulatedAprilTagDetector started, {len(self._tags)} tags loaded"
        )

    def _load_map(self) -> None:
        path = os.path.join(self._maps_path, "default.json")
        if not os.path.exists(path):
            self.get_logger().warn(f"Map file not found: {path}")
            return
        try:
            with open(path, "r") as f:
                data = json.load(f)
            self._tags = {str(k): v for k, v in data.get("tags", {}).items()}
        except Exception as e:
            self.get_logger().error(f"Failed to load map: {e}")

    def update_robot_pose(self, x: float, y: float, yaw: float) -> None:
        """Called externally when new robot pose is available."""
        self._robot_x = x
        self._robot_y = y
        self._robot_yaw = yaw

    def _detect_tags(self) -> None:
        msg = AprilTagDetections()
        now = self.get_clock().now()
        msg.timestamp = TimeMsg(sec=int(now.seconds_nanoseconds()[0]),
                                nanosec=int(now.seconds_nanoseconds()[1]))
        msg.frame_id = "camera_link"

        camera_x = self._robot_x
        camera_y = self._robot_y
        camera_yaw = self._robot_yaw

        for tag_id_str, tag_info in self._tags.items():
            tag_id = int(tag_id_str)
            tx = tag_info.get("x", 0.0)
            ty = tag_info.get("y", 0.0)

            # Vector from camera to tag
            dx = tx - camera_x
            dy = ty - camera_y
            dist = math.sqrt(dx * dx + dy * dy)

            if dist < self._min_dist or dist > self._max_dist:
                continue

            # Angle to tag in robot frame
            angle_to_tag = math.atan2(dy, dx)
            relative_angle = angle_to_tag - camera_yaw
            # Normalize to [-pi, pi]
            while relative_angle > math.pi:
                relative_angle -= 2 * math.pi
            while relative_angle < -math.pi:
                relative_angle += 2 * math.pi

            # Check horizontal FOV
            if abs(relative_angle) > self._fov_h:
                continue

            # Assume tag is at ground level, camera at 0.3m height
            height_diff = 0.3  # camera height above ground
            vertical_angle = math.atan2(height_diff, dist)
            if abs(vertical_angle) > self._fov_v:
                continue

            # Add noise
            noisy_offset_x = (relative_angle / self._fov_h) + random.gauss(0, self._noise_offset)
            noisy_offset_y = random.gauss(0, self._noise_offset)
            noisy_dist = dist + random.gauss(0, self._noise_distance)
            noisy_dist = max(noisy_dist, 0.01)
            noisy_yaw = math.degrees(relative_angle) + random.gauss(0, self._noise_angle)

            det = AprilTagDetection()
            det.id = tag_id
            det.distance = float(noisy_dist * 1000.0)  # convert m to mm
            det.center_offset_x = float(max(-1.0, min(1.0, noisy_offset_x)))
            det.center_offset_y = float(max(-1.0, min(1.0, noisy_offset_y)))
            det.yaw = float(noisy_yaw)
            det.pitch = 0.0
            det.roll = 0.0
            det.hamming = 0

            msg.detections.append(det)

        self._pub.publish(msg)


def main() -> None:
    rclpy.init()
    import os
    maps_dir = os.path.join(os.getcwd(), "config", "maps")
    node = SimulatedAprilTagDetector(maps_path=maps_dir)
    try:
        rclpy.spin(node)
    except KeyboardInterrupt:
        pass
    finally:
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
