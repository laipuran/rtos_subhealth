import math
import threading
from typing import List, Optional, Tuple

import cv2
import numpy as np
import rclpy
from cv_bridge import CvBridge
from rclpy.node import Node
from sensor_msgs.msg import Image

from apriltag_interfaces.msg import AprilTagDetection, AprilTagDetections

try:
    from pupil_apriltags import Detector
except ImportError:  # pragma: no cover - runtime dependency check
    Detector = None


def rotation_matrix_to_rpy_degrees(rot: np.ndarray) -> Tuple[float, float, float]:
    """Convert a 3x3 rotation matrix to roll, yaw, pitch in degrees."""
    pitch = math.asin(-float(np.clip(rot[2, 0], -1.0, 1.0)))
    roll = math.atan2(float(rot[2, 1]), float(rot[2, 2]))
    yaw = math.atan2(float(rot[1, 0]), float(rot[0, 0]))
    return math.degrees(roll), math.degrees(yaw), math.degrees(pitch)


class AprilTagPerceptionNode(Node):
    def __init__(self) -> None:
        super().__init__("apriltag_perception_node")

        self.declare_parameter("input_image_topic", "/camera/device")
        self.declare_parameter("output_topic", "/perception/apriltag_detections/device")
        self.declare_parameter("frame_id", "device")
        self.declare_parameter("publish_rate_hz", 10.0)
        self.declare_parameter("tag_family", "tag36h11")
        self.declare_parameter("tag_size_m", 0.16)
        self.declare_parameter("hamming_max", 2)
        self.declare_parameter("camera_fx", 0.0)
        self.declare_parameter("camera_fy", 0.0)
        self.declare_parameter("camera_cx", 0.0)
        self.declare_parameter("camera_cy", 0.0)

        self.input_image_topic = self.get_parameter("input_image_topic").value
        self.output_topic = self.get_parameter("output_topic").value
        self.frame_id = self.get_parameter("frame_id").value
        self.publish_rate_hz = float(self.get_parameter("publish_rate_hz").value)
        self.tag_family = self.get_parameter("tag_family").value
        self.tag_size_m = float(self.get_parameter("tag_size_m").value)
        self.hamming_max = int(self.get_parameter("hamming_max").value)
        self.camera_fx = float(self.get_parameter("camera_fx").value)
        self.camera_fy = float(self.get_parameter("camera_fy").value)
        self.camera_cx = float(self.get_parameter("camera_cx").value)
        self.camera_cy = float(self.get_parameter("camera_cy").value)

        if self.publish_rate_hz < 10.0:
            self.get_logger().warn("publish_rate_hz < 10, forcing to 10.0 to satisfy RFC")
            self.publish_rate_hz = 10.0

        if Detector is None:
            raise RuntimeError(
                "pupil_apriltags is not installed. Install it before starting the node."
            )

        self.detector = Detector(families=self.tag_family)
        self.bridge = CvBridge()
        self.lock = threading.Lock()
        self.latest_image: Optional[np.ndarray] = None

        self.publisher = self.create_publisher(AprilTagDetections, self.output_topic, 10)
        self.subscription = self.create_subscription(
            Image, self.input_image_topic, self._image_callback, 10
        )
        self.timer = self.create_timer(1.0 / self.publish_rate_hz, self._publish_tick)

        self.get_logger().info(
            f"AprilTag node started. input={self.input_image_topic}, output={self.output_topic}, "
            f"rate={self.publish_rate_hz:.1f}Hz"
        )

    def _image_callback(self, msg: Image) -> None:
        try:
            frame = self.bridge.imgmsg_to_cv2(msg, desired_encoding="bgr8")
        except Exception as exc:  # pragma: no cover - runtime safety path
            self.get_logger().warn(f"Failed to convert image: {exc}")
            return

        with self.lock:
            self.latest_image = frame

    def _publish_tick(self) -> None:
        msg = AprilTagDetections()
        msg.timestamp = self.get_clock().now().to_msg()
        msg.frame_id = self.frame_id

        with self.lock:
            frame = None if self.latest_image is None else self.latest_image.copy()

        if frame is None:
            self.publisher.publish(msg)
            return

        detections = self._run_detection(frame)
        msg.detections = detections
        self.publisher.publish(msg)

    def _run_detection(self, frame: np.ndarray) -> List[AprilTagDetection]:
        gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)

        use_pose = all(v > 0.0 for v in [self.camera_fx, self.camera_fy, self.camera_cx, self.camera_cy])
        detect_args = {
            "estimate_tag_pose": use_pose,
            "tag_size": self.tag_size_m,
        }
        if use_pose:
            detect_args["camera_params"] = (
                self.camera_fx,
                self.camera_fy,
                self.camera_cx,
                self.camera_cy,
            )

        try:
            raw_detections = self.detector.detect(gray, **detect_args)
        except Exception as exc:  # pragma: no cover - runtime safety path
            self.get_logger().warn(f"Detector error: {exc}")
            return []

        height, width = gray.shape[:2]
        output: List[AprilTagDetection] = []

        for det in raw_detections:
            hamming = int(det.hamming)
            if hamming > self.hamming_max:
                continue

            center_x = float(det.center[0])
            center_y = float(det.center[1])

            # Normalize pixel offsets to [-1, 1] range required by RFC.
            offset_x = (center_x - (width / 2.0)) / (width / 2.0)
            offset_y = (center_y - (height / 2.0)) / (height / 2.0)
            offset_x = max(-1.0, min(1.0, offset_x))
            offset_y = max(-1.0, min(1.0, offset_y))

            distance_mm = -1.0
            roll = 0.0
            yaw = 0.0
            pitch = 0.0

            if use_pose and hasattr(det, "pose_t") and hasattr(det, "pose_R"):
                pose_t = np.asarray(det.pose_t, dtype=np.float64).reshape(-1)
                distance_mm = float(np.linalg.norm(pose_t) * 1000.0)
                rot = np.asarray(det.pose_R, dtype=np.float64).reshape(3, 3)
                roll, yaw, pitch = rotation_matrix_to_rpy_degrees(rot)

            out = AprilTagDetection()
            out.id = int(det.tag_id)
            out.distance = float(distance_mm)
            out.center_offset_x = float(offset_x)
            out.center_offset_y = float(offset_y)
            out.roll = float(roll)
            out.yaw = float(yaw)
            out.pitch = float(pitch)
            out.hamming = hamming
            output.append(out)

        return output


def main(args=None) -> None:
    rclpy.init(args=args)
    node = AprilTagPerceptionNode()
    try:
        rclpy.spin(node)
    finally:
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
