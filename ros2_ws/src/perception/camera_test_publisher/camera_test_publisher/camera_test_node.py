from typing import Optional, Tuple

import cv2
import rclpy
from cv_bridge import CvBridge
from rclpy.node import Node
from sensor_msgs.msg import CameraInfo, Image


class CameraTestPublisherNode(Node):
    def __init__(self) -> None:
        super().__init__("camera_test_node")

        self.declare_parameter("output_topic", "/camera/test")
        self.declare_parameter("camera_info_topic", "")
        self.declare_parameter("frame_id", "camera_test_frame")
        self.declare_parameter("camera_index", 0)
        self.declare_parameter("fps", 30.0)
        self.declare_parameter("width", 640)
        self.declare_parameter("height", 480)

        self.output_topic = str(self.get_parameter("output_topic").value)
        camera_info_topic = str(self.get_parameter("camera_info_topic").value)
        self.camera_info_topic = (
            camera_info_topic if camera_info_topic else f"{self.output_topic}/camera_info"
        )
        self.frame_id = str(self.get_parameter("frame_id").value)
        self.camera_index = int(self.get_parameter("camera_index").value)
        self.fps = max(1.0, float(self.get_parameter("fps").value))
        self.width = int(self.get_parameter("width").value)
        self.height = int(self.get_parameter("height").value)

        self.bridge = CvBridge()
        self.publisher = self.create_publisher(Image, self.output_topic, 10)
        self.camera_info_publisher = self.create_publisher(
            CameraInfo, self.camera_info_topic, 10
        )
        self.capture: Optional[cv2.VideoCapture] = cv2.VideoCapture(
            self.camera_index, cv2.CAP_V4L2
        )

        if not self.capture.isOpened():
            raise RuntimeError(
                f"Failed to open camera index {self.camera_index}. "
                "Check device availability and permissions."
            )

        self.capture.set(
            cv2.CAP_PROP_FOURCC, cv2.VideoWriter_fourcc(*"MJPG")
        )
        self.capture.set(cv2.CAP_PROP_FRAME_WIDTH, float(self.width))
        self.capture.set(cv2.CAP_PROP_FRAME_HEIGHT, float(self.height))
        self.capture.set(cv2.CAP_PROP_FPS, float(self.fps))

        self.timer = self.create_timer(1.0 / self.fps, self._publish_frame)

        self.get_logger().info(
            f"Camera test publisher started. topic={self.output_topic}, "
            f"camera_info_topic={self.camera_info_topic}, "
            f"camera_index={self.camera_index}, fps={self.fps:.1f}, "
            f"size={self.width}x{self.height}"
        )

    def _camera_intrinsics_from_device(self, width: int, height: int) -> Tuple[float, float, float, float]:
        # CameraInfo should come from camera-native properties rather than YAML overrides.
        focal = 0.0
        if hasattr(cv2, "CAP_PROP_FOCAL_LENGTH"):
            focal = float(self.capture.get(cv2.CAP_PROP_FOCAL_LENGTH))

        if focal <= 0.0:
            focal = float(max(width, height))

        cx = float(width) / 2.0
        cy = float(height) / 2.0
        return float(focal), float(focal), cx, cy

    def _build_camera_info(self, stamp, width: int, height: int) -> CameraInfo:
        fx, fy, cx, cy = self._camera_intrinsics_from_device(width, height)

        info = CameraInfo()
        info.header.stamp = stamp
        info.header.frame_id = self.frame_id
        info.width = int(width)
        info.height = int(height)
        info.distortion_model = "plumb_bob"
        info.d = [0.0, 0.0, 0.0, 0.0, 0.0]
        info.k = [
            float(fx),
            0.0,
            float(cx),
            0.0,
            float(fy),
            float(cy),
            0.0,
            0.0,
            1.0,
        ]
        info.r = [
            1.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            1.0,
        ]
        info.p = [
            float(fx),
            0.0,
            float(cx),
            0.0,
            0.0,
            float(fy),
            float(cy),
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ]
        return info

    def _publish_frame(self) -> None:
        if self.capture is None:
            return

        ok, frame = self.capture.read()
        if not ok or frame is None:
            self.get_logger().warn("Failed to capture frame from camera.")
            return

        height, width = frame.shape[:2]
        stamp = self.get_clock().now().to_msg()

        msg = self.bridge.cv2_to_imgmsg(frame, encoding="bgr8")
        msg.header.stamp = stamp
        msg.header.frame_id = self.frame_id
        self.publisher.publish(msg)

        camera_info = self._build_camera_info(stamp, width, height)
        self.camera_info_publisher.publish(camera_info)

    def destroy_node(self) -> bool:
        if self.capture is not None:
            self.capture.release()
            self.capture = None
        return super().destroy_node()


def main(args=None) -> None:
    rclpy.init(args=args)
    node = CameraTestPublisherNode()
    try:
        rclpy.spin(node)
    finally:
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
