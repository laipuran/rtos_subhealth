from __future__ import annotations

from typing import Optional

import rclpy
from rclpy.action import ActionServer, CancelResponse, GoalResponse
from rclpy.node import Node

from ros_interfaces.action import ExecTask
from ros_interfaces.msg import Segment
from ros_interfaces.srv import PlanPath


class ExecLayerNode(Node):
    """Skeleton exec layer node implementing RFC003/004 behaviors."""

    def __init__(self) -> None:
        super().__init__("exec_layer_node")
        self._planner_client = self.create_client(PlanPath, "plan_path")
        self._action_server = ActionServer(
            self,
            ExecTask,
            "exec_task",
            execute_callback=self.execute_task,
            goal_callback=self.handle_goal,
            cancel_callback=self.handle_cancel,
        )

    def handle_goal(self, goal_request: ExecTask.Goal) -> GoalResponse:
        """Validate incoming goal fields and accept or reject."""
        if not goal_request.type:
            self.get_logger().warn("Rejecting goal: missing type")
            return GoalResponse.REJECT
        return GoalResponse.ACCEPT

    def handle_cancel(self, goal_handle) -> CancelResponse:
        """Handle action cancel requests from decision layer."""
        self.get_logger().info("Cancel requested")
        return CancelResponse.ACCEPT

    def execute_task(self, goal_handle):
        """Dispatch task to planner, then drive execution loop."""
        goal = goal_handle.request
        feedback = ExecTask.Feedback()
        feedback.state = "accepted"
        feedback.progress = 0.0
        feedback.current_tag = -1
        feedback.next_tag = -1
        goal_handle.publish_feedback(feedback)

        plan = self._request_plan(goal)
        if plan is None:
            return self._finish_with_error(goal_handle, "INTERNAL", "planner unavailable")

        feedback.state = "running"
        feedback.next_tag = plan.next_tag
        goal_handle.publish_feedback(feedback)

        self._execute_segments(plan.segments, goal_handle)

        result = ExecTask.Result()
        result.final_state = "succeeded"
        result.error_code = ""
        result.message = ""
        result.finished_time = self.get_clock().now().to_msg()
        goal_handle.succeed()
        return result

    def _request_plan(self, goal: ExecTask.Goal) -> Optional[PlanPath.Response]:
        """Send PlanPath request to planner and wait for response."""
        if not self._planner_client.wait_for_service(timeout_sec=1.0):
            self.get_logger().error("Planner service not available")
            return None

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

    def _execute_segments(self, segments: list[Segment], goal_handle) -> None:
        """Placeholder to iterate segments and drive robot motion."""
        for _segment in segments:
            if goal_handle.is_cancel_requested:
                self._finish_canceled(goal_handle)
                return
            self._drive_segment(_segment)

    def _drive_segment(self, segment: Segment) -> None:
        """Execute a single segment; replace with real motion control."""
        self.get_logger().info(
            f"Driving segment {segment.from_tag}->{segment.to_tag}"
        )

    def _finish_with_error(self, goal_handle, code: str, message: str):
        """Finish action with failure status and error payload."""
        result = ExecTask.Result()
        result.final_state = "failed"
        result.error_code = code
        result.message = message
        result.finished_time = self.get_clock().now().to_msg()
        goal_handle.abort()
        return result

    def _finish_canceled(self, goal_handle) -> ExecTask.Result:
        """Finish action with canceled result."""
        result = ExecTask.Result()
        result.final_state = "canceled"
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
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
