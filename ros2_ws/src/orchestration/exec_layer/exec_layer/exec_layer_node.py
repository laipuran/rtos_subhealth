from __future__ import annotations

import json
import math
import os
import threading
import time
from typing import Optional

import rclpy
from rclpy.action import ActionServer, CancelResponse, GoalResponse
from rclpy.node import Node
from rclpy.qos import QoSProfile, ReliabilityPolicy

from ros_interfaces.action import ExecTask
from ros_interfaces.msg import Segment
from ros_interfaces.srv import PlanPath

from apriltag_interfaces.msg import AprilTagDetections

from .robot import RobotInterface, SportClientRobot, SimRobot
from .log_util import info as log_info, warn as log_warn, error as log_error

_STATE_RUNNING = "running"
_STATE_SUCCEEDED = "succeeded"
_STATE_FAILED = "failed"
_STATE_CANCELED = "canceled"

_GO_TO_TAG = "go_to_tag"
_PATROL_ROUTE = "patrol_route"


class ExecLayerNode(Node):
    """Exec layer node implementing RFC003/004 with RobotInterface."""

    def __init__(self) -> None:
        super().__init__("exec_layer_node")

        self._cancel_requested = False
        self._robot: Optional[RobotInterface] = None
        self._apriltag_sub = None
        self._latest_detections: Optional[AprilTagDetections] = None
        self._detection_lock = threading.Lock()

        # Declare parameters
        robot_backend = self.declare_parameter("robot_backend", "mock").value
        dds_domain_id = self.declare_parameter("dds_domain_id", 1).value
        dds_interface = self.declare_parameter("dds_interface", "lo").value
        maps_dir = self.declare_parameter("maps_dir", "").value

        # Initialize robot backend
        if robot_backend == "real":
            self._robot = SportClientRobot(domain_id=dds_domain_id, interface=dds_interface)
            if not self._robot.connect():
                self.get_logger().error("SportClientRobot connect failed")
        elif robot_backend == "sim":
            if not maps_dir:
                maps_dir = os.path.join(os.getcwd(), "config", "maps")
            self._robot = SimRobot(maps_path=maps_dir)
            if not self._robot.connect():
                self.get_logger().error("SimRobot connect failed")
        else:
            self._robot = None
            self.get_logger().info(f"robot_backend={robot_backend}, using stub behavior")

        # Subscribe to AprilTag detections
        self._apriltag_sub = self.create_subscription(
            AprilTagDetections,
            "/perception/apriltag_detections",
            self._apriltag_callback,
            QoSProfile(depth=10, reliability=ReliabilityPolicy.BEST_EFFORT),
        )

        # Planner client
        self._planner_client = self.create_client(PlanPath, "plan_path")

        # Action server
        self._action_server = ActionServer(
            self,
            ExecTask,
            "exec_task",
            execute_callback=self.execute_task,
            goal_callback=self.handle_goal,
            cancel_callback=self.handle_cancel,
        )

        self._robot_backend = robot_backend

        self.get_logger().info(f"Exec Layer ready, backend={robot_backend}")

    def handle_goal(self, goal_request: ExecTask.Goal) -> GoalResponse:
        if not goal_request.type:
            self.get_logger().warn("Rejecting goal: missing type")
            return GoalResponse.REJECT
        return GoalResponse.ACCEPT

    def handle_cancel(self, goal_handle) -> CancelResponse:
        self.get_logger().info("Cancel requested")
        self._cancel_requested = True
        if self._robot and self._robot.is_connected():
            self._robot.damp()
        return CancelResponse.ACCEPT

    def _apriltag_callback(self, msg: AprilTagDetections) -> None:
        with self._detection_lock:
            self._latest_detections = msg

    def execute_task(self, goal_handle):
        self._cancel_requested = False
        goal = goal_handle.request
        feedback_msg = ExecTask.Feedback()
        feedback_msg.state = "accepted"
        feedback_msg.progress = 0.0
        feedback_msg.current_tag = -1
        feedback_msg.next_tag = -1
        goal_handle.publish_feedback(feedback_msg)

        plan = self._request_plan(goal)
        if plan is None:
            if self._robot_backend == "mock":
                log_info("exec", "TASK", "mock fallback: no planner")
                self._execute_segments([], goal, feedback_msg, goal_handle)
                result = ExecTask.Result()
                result.final_state = _STATE_SUCCEEDED
                result.error_code = ""
                result.message = "mock: planner unavailable, simulated success"
                result.finished_time = self.get_clock().now().to_msg()
                goal_handle.succeed()
                return result
            return self._finish_with_error(goal_handle, "INTERNAL", "planner unavailable")

        feedback_msg.state = _STATE_RUNNING
        feedback_msg.next_tag = plan.next_tag
        goal_handle.publish_feedback(feedback_msg)

        self._execute_segments(plan.segments, goal, feedback_msg, goal_handle)

        result = ExecTask.Result()
        result.final_state = _STATE_SUCCEEDED
        result.error_code = ""
        result.message = ""
        result.finished_time = self.get_clock().now().to_msg()
        goal_handle.succeed()
        return result

    def _request_plan(self, goal: ExecTask.Goal) -> Optional[PlanPath.Response]:
        if not self._planner_client.wait_for_service(timeout_sec=1.0):
            log_error("exec", "PLAN", "planner service unavailable")
            return None

        log_info("exec", "PLAN", f"request: {goal.type} tags={list(goal.target_tags)}")
        request = PlanPath.Request()
        request.goal_id = ""
        request.task_type = goal.type
        request.route_id = goal.route_id
        request.target_tags = goal.target_tags
        request.start_tag = -1
        request.constraints = goal.constraints
        request.deadline_ms = goal.deadline_ms
        request.allow_partial = False
        request.replan_reason = ""

        future = self._planner_client.call_async(request)
        rclpy.spin_until_future_complete(self, future)
        if future.result() is None:
            self.get_logger().error("Planner call failed")
            return None
        return future.result()

    def _execute_segments(
        self,
        segments: list[Segment],
        goal: ExecTask.Goal,
        feedback: ExecTask.Feedback,
        goal_handle,
    ) -> None:
        total = len(segments)
        if total > 0:
            route = "→".join(str(s.from_tag) for s in segments)
            log_info("exec", "DRIVE", f"{total} segments: {route}→{segments[-1].to_tag}")
        for idx, segment in enumerate(segments):
            if self._cancel_requested or goal_handle.is_cancel_requested:
                self._finish_canceled(goal_handle)
                return

            feedback.progress = idx / total if total > 0 else 1.0
            feedback.current_tag = segment.from_tag
            feedback.next_tag = segment.to_tag
            goal_handle.publish_feedback(feedback)

            success = self._drive_segment(segment, goal)
            if not success:
                if not self._cancel_requested:
                    self._finish_with_error(goal_handle, "UNREACHABLE",
                                            f"failed to reach tag {segment.to_tag}")
                return

        feedback.progress = 1.0
        feedback.current_tag = segments[-1].to_tag if segments else -1
        feedback.next_tag = -1
        goal_handle.publish_feedback(feedback)

    def _drive_segment(self, segment: Segment, goal: ExecTask.Goal) -> bool:
        to_tag = segment.to_tag
        from_tag = segment.from_tag
        dist = 0.0
        if self._robot and isinstance(self._robot, SimRobot):
            # 从地图获取距离
            maps_dir = self._maps_dir if hasattr(self, '_maps_dir') else os.path.join(os.getcwd(), "config", "maps")
            map_path = os.path.join(maps_dir, "default.json")
            if os.path.exists(map_path):
                with open(map_path) as f:
                    data = json.load(f)
                tags = data.get("tags", {})
                fpos = tags.get(str(from_tag), {"x": 0, "y": 0})
                tpos = tags.get(str(to_tag), {"x": 0, "y": 0})
                dist = math.sqrt((tpos["x"] - fpos["x"])**2 + (tpos["y"] - fpos["y"])**2)
        log_info("exec", "DRIVE", f"segment {from_tag}→{to_tag}" +
                 (f", dist={dist:.1f}m" if dist else ""))

        if not self._robot or not self._robot.is_connected():
            self.get_logger().info("No robot backend, simulating segment completion")
            time.sleep(1.0)
            return True

        # 如果是 SimRobot，获取 tag 坐标并计算方向
        if isinstance(self._robot, SimRobot):
            self._robot.stand_up()
            time.sleep(1.0)
            maps_dir = self.declare_parameter("maps_dir", "").value
            if not maps_dir:
                maps_dir = os.path.join(os.getcwd(), "config", "maps")
            map_path = os.path.join(maps_dir, "default.json")
            tags = {}
            if os.path.exists(map_path):
                with open(map_path) as f:
                    data = json.load(f)
                tags = data.get("tags", {})

            from_pos = tags.get(str(from_tag), {"x": 0, "y": 0})
            to_pos = tags.get(str(to_tag), {"x": 0, "y": 0})
            dx = to_pos["x"] - from_pos["x"]
            dy = to_pos["y"] - from_pos["y"]
            dist = math.sqrt(dx ** 2 + dy ** 2)
            speed = 0.3
            duration = max(dist / speed, 3.0)

            self._robot.move_blocking(0.3, 0.0, 0.0, duration)
            self._robot.damp()

            # 检查是否到达
            if self._robot.check_tag_arrival(to_tag):
                log_info("exec", "DRIVE", f"→ tag {to_tag} reached")
                return True
            log_warn("exec", "DRIVE", f"→ tag {to_tag} NOT reached")
            return False

        # 真机模式用 AprilTag 检测
        self._robot.stand_up()
        time.sleep(0.5)
        self._robot.move(0.3, 0.0, 0.0)

        deadline = goal.deadline_ms / 1000.0 if goal.deadline_ms > 0 else 15.0
        start_time = time.time()

        while time.time() - start_time < deadline:
            if self._cancel_requested:
                self._robot.damp()
                return False

            with self._detection_lock:
                detections = self._latest_detections

            if detections is not None:
                for det in detections.detections:
                    if det.id == to_tag:
                        if (abs(det.center_offset_x) < 0.05
                                and abs(det.center_offset_y) < 0.05
                                and det.distance < 500.0):
                            log_info("exec", "DRIVE",
                                     f"→ tag {to_tag} reached (dist={det.distance:.0f}mm)")
                            self._robot.damp()
                            return True
            time.sleep(0.1)

        log_warn("exec", "DRIVE", f"→ tag {to_tag} timeout")
        self._robot.damp()
        return False

    def _finish_with_error(
        self, goal_handle, code: str, message: str
    ) -> ExecTask.Result:
        result = ExecTask.Result()
        result.final_state = _STATE_FAILED
        result.error_code = code
        result.message = message
        result.finished_time = self.get_clock().now().to_msg()
        goal_handle.abort()
        return result

    def _finish_canceled(self, goal_handle) -> ExecTask.Result:
        result = ExecTask.Result()
        result.final_state = _STATE_CANCELED
        result.error_code = ""
        result.message = ""
        result.finished_time = self.get_clock().now().to_msg()
        goal_handle.canceled()
        return result


def main() -> None:
    rclpy.init()
    node = ExecLayerNode()
    try:
        rclpy.spin(node)
    except KeyboardInterrupt:
        pass
    finally:
        if node._robot and node._robot.is_connected():
            node._robot.damp()
            node._robot.disconnect()
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
