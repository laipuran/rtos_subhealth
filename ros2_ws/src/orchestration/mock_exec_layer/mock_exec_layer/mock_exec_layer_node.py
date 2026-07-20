from __future__ import annotations

import time

import rclpy
from rclpy.action import ActionServer, CancelResponse, GoalResponse
from rclpy.node import Node

from ros_interfaces.action import ExecTask


class MockExecLayerNode(Node):
    def __init__(self) -> None:
        super().__init__("mock_exec_layer_node")

        action_name = self.declare_parameter("action_name", "mock_exec_task").value
        self._cancel_requested = False

        self._action_server = ActionServer(
            self,
            ExecTask,
            action_name,
            execute_callback=self.execute_task,
            goal_callback=self.handle_goal,
            cancel_callback=self.handle_cancel,
        )

        self.get_logger().info(
            f"Mock Exec Layer ready on action: {action_name}"
        )

    def handle_goal(self, goal_request: ExecTask.Goal) -> GoalResponse:
        if not goal_request.type:
            self.get_logger().warn("Rejecting goal: missing type")
            return GoalResponse.REJECT
        return GoalResponse.ACCEPT

    def handle_cancel(self, goal_handle) -> CancelResponse:
        self.get_logger().info("Cancel requested")
        self._cancel_requested = True
        return CancelResponse.ACCEPT

    def execute_task(self, goal_handle):
        self._cancel_requested = False
        goal = goal_handle.request
        feedback = ExecTask.Feedback()

        feedback.state = "running"
        feedback.progress = 0.0
        feedback.current_tag = -1
        feedback.next_tag = -1
        goal_handle.publish_feedback(feedback)
        self.get_logger().info(f"Starting mock task: type={goal.type}, targets={goal.target_tags}")

        # Simulate failure when max_speed_mps < 0
        if goal.constraints.max_speed_mps < 0.0:
            result = ExecTask.Result()
            result.final_state = "failed"
            result.error_code = "SIMULATED_FAILURE"
            result.message = f"mock: max_speed_mps={goal.constraints.max_speed_mps} triggered failure"
            result.finished_time = self.get_clock().now().to_msg()
            goal_handle.abort()
            self.get_logger().warn(result.message)
            return result

        if goal.type == "hold":
            return self._immediate_success(goal_handle, goal)

        if goal.type == "go_to_tag":
            return self._mock_go_to_tag(goal_handle, goal)

        if goal.type == "patrol_route":
            return self._mock_patrol_route(goal_handle, goal)

        result = ExecTask.Result()
        result.final_state = "failed"
        result.error_code = "UNSUPPORTED_TYPE"
        result.message = f"mock: unknown type {goal.type}"
        result.finished_time = self.get_clock().now().to_msg()
        goal_handle.abort()
        return result

    def _immediate_success(self, goal_handle, goal: ExecTask.Goal) -> ExecTask.Result:
        feedback = ExecTask.Feedback()
        feedback.state = "running"
        feedback.progress = 1.0
        feedback.current_tag = -1
        feedback.next_tag = -1
        feedback.error_code = ""
        feedback.message = "hold: no movement needed"
        goal_handle.publish_feedback(feedback)

        result = ExecTask.Result()
        result.final_state = "succeeded"
        result.error_code = ""
        result.message = "hold completed"
        result.finished_time = self.get_clock().now().to_msg()
        goal_handle.succeed()
        return result

    def _mock_go_to_tag(self, goal_handle, goal: ExecTask.Goal) -> ExecTask.Result:
        target = goal.target_tags[0] if goal.target_tags else 42
        steps = 3
        feedback = ExecTask.Feedback()

        for i in range(1, steps + 1):
            if self._cancel_requested or goal_handle.is_cancel_requested:
                return self._finish_canceled(goal_handle)

            time.sleep(1)

            feedback.state = "running"
            feedback.progress = i / steps
            feedback.current_tag = -1 if i == 1 else target
            feedback.next_tag = target if i < steps else -1
            feedback.error_code = ""
            feedback.message = f"approaching tag {target} (step {i}/{steps})"
            goal_handle.publish_feedback(feedback)
            self.get_logger().info(f"mock go_to_tag: {i}/{steps}")

        result = ExecTask.Result()
        result.final_state = "succeeded"
        result.error_code = ""
        result.message = f"reached tag {target}"
        result.finished_time = self.get_clock().now().to_msg()
        goal_handle.succeed()
        return result

    def _mock_patrol_route(self, goal_handle, goal: ExecTask.Goal) -> ExecTask.Result:
        tags = goal.target_tags
        if not tags:
            tags = [10, 20, 30]

        feedback = ExecTask.Feedback()
        total = len(tags)

        for idx, tag in enumerate(tags):
            if self._cancel_requested or goal_handle.is_cancel_requested:
                return self._finish_canceled(goal_handle)

            time.sleep(1)

            feedback.state = "running"
            feedback.progress = (idx + 1) / total
            feedback.current_tag = tag
            feedback.next_tag = tags[idx + 1] if idx + 1 < len(tags) else -1
            feedback.error_code = ""
            feedback.message = f"patrolled tag {tag} ({idx + 1}/{total})"
            goal_handle.publish_feedback(feedback)
            self.get_logger().info(f"mock patrol: tag {tag} ({idx + 1}/{total})")

        result = ExecTask.Result()
        result.final_state = "succeeded"
        result.error_code = ""
        result.message = f"patrol route completed: {len(tags)} tags"
        result.finished_time = self.get_clock().now().to_msg()
        goal_handle.succeed()
        return result

    def _finish_canceled(self, goal_handle) -> ExecTask.Result:
        result = ExecTask.Result()
        result.final_state = "canceled"
        result.error_code = ""
        result.message = "task canceled by user"
        result.finished_time = self.get_clock().now().to_msg()
        goal_handle.canceled()
        self.get_logger().info("mock task canceled")
        return result


def main() -> None:
    rclpy.init()
    node = MockExecLayerNode()
    try:
        rclpy.spin(node)
    except KeyboardInterrupt:
        pass
    finally:
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
