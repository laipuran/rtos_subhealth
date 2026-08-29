from __future__ import annotations

import random
import threading
from typing import Dict, List

from rclpy.node import Node
from rclpy.qos import QoSProfile, ReliabilityPolicy, HistoryPolicy
from builtin_interfaces.msg import Time
from physio_interfaces.msg import PhysioSample

_SENSORS: List[Dict] = [
    {"data_src": "mock_spo2", "data_type": "spo2", "base": 97.0, "noise": 1.0},
    {"data_src": "mock_heart_rate", "data_type": "heart_rate", "base": 75.0, "noise": 5.0},
    {"data_src": "mock_bp_systolic", "data_type": "systolic_mmhg", "base": 120.0, "noise": 4.0},
    {"data_src": "mock_bp_diastolic", "data_type": "diastolic_mmhg", "base": 78.0, "noise": 3.0},
    {"data_src": "mock_body_temp", "data_type": "body_temp_c", "base": 36.6, "noise": 0.2},
    {"data_src": "mock_respiratory_rate", "data_type": "respiratory_rate", "base": 16.0, "noise": 2.0},
]


class PhysioMockPublisher(Node):
    def __init__(self) -> None:
        super().__init__("physio_mock_publisher")
        self._scenario = self.declare_parameter("scenario", "normal").value
        self._rate_hz = self.declare_parameter("rate_hz", 1.0).value

        qos = QoSProfile(
            reliability=ReliabilityPolicy.RELIABLE,
            history=HistoryPolicy.KEEP_LAST,
            depth=10,
        )
        self._pubs = {}
        for s in _SENSORS:
            topic = f"/physio/{s['data_src']}"
            self._pubs[s["data_src"]] = self.create_publisher(PhysioSample, topic, qos)
            self.get_logger().info(f"Publishing {s['data_type']} on {topic}")

        self._timer = self.create_timer(1.0 / self._rate_hz, self._tick)
        self._lock = threading.Lock()

    def _value(self, spec: Dict) -> float:
        val = spec["base"] + random.uniform(-spec["noise"], spec["noise"])
        if self._scenario == "anomaly" and spec["data_type"] == "spo2":
            val = 85.0 + random.uniform(-1.0, 1.0)
        return round(val, 2)

    def _tick(self) -> None:
        now = self.get_clock().now().to_msg()
        with self._lock:
            for spec in _SENSORS:
                msg = PhysioSample()
                msg.timestamp = now
                msg.data_src = spec["data_src"]
                msg.data_type = spec["data_type"]
                msg.data = self._value(spec)
                msg.valid = True
                self._pubs[spec["data_src"]].publish(msg)


def main() -> None:
    import rclpy

    rclpy.init()
    node = PhysioMockPublisher()
    try:
        rclpy.spin(node)
    except KeyboardInterrupt:
        pass
    finally:
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
