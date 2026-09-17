"""ROS 2 action server for deterministic task execution."""

import time

import rclpy
from rclpy.action import ActionServer, CancelResponse, GoalResponse
from rclpy.callback_groups import ReentrantCallbackGroup
from rclpy.executors import MultiThreadedExecutor
from rclpy.node import Node
from ros_interfaces.action import ExecTask

from .logic import (
    route_for,
    step_feedback,
    terminal_feedback,
    UnsupportedTask,
    validate_step_delay,
)


class MockExecLayerNode(Node):
    """Serve predictable execution results for integration testing."""

    def __init__(self) -> None:
        super().__init__('mock_exec_layer')
        self.declare_parameter('action_name', 'mock_exec_task')
        self.declare_parameter('step_delay_s', 1.0)

        action_name = self.get_parameter('action_name').value
        self._step_delay_s = validate_step_delay(
            float(self.get_parameter('step_delay_s').value)
        )
        self._action_server = ActionServer(
            self,
            ExecTask,
            action_name,
            execute_callback=self.execute_callback,
            goal_callback=self.goal_callback,
            cancel_callback=self.cancel_callback,
            callback_group=ReentrantCallbackGroup(),
        )

    def goal_callback(self, goal_request: ExecTask.Goal) -> GoalResponse:
        if not goal_request.type:
            return GoalResponse.REJECT
        return GoalResponse.ACCEPT

    def cancel_callback(self, _goal_handle) -> CancelResponse:
        return CancelResponse.ACCEPT

    def execute_callback(self, goal_handle) -> ExecTask.Result:
        request = goal_handle.request
        if request.constraints.max_speed_mps < 0.0:
            goal_handle.abort()
            return self._result('failed', 'SIMULATED_FAILURE', 'Negative max speed')

        try:
            route = route_for(request.type, request.target_tags)
        except UnsupportedTask:
            goal_handle.abort()
            return self._result('failed', 'UNSUPPORTED_TYPE', 'Unsupported task type')

        finished_stages = 0
        if self._cancel_requested(goal_handle):
            return self._cancel(goal_handle, route, finished_stages)

        if not route:
            self._publish_feedback(
                goal_handle,
                'completed',
                route,
                terminal_feedback(route, finished_stages),
            )

        for index in range(len(route)):
            if self._step_delay_s > 0.0:
                time.sleep(self._step_delay_s)
            if self._cancel_requested(goal_handle):
                return self._cancel(goal_handle, route, finished_stages)

            step = step_feedback(route, index)
            self._publish_feedback(goal_handle, 'executing', route, step)
            finished_stages = step.finished_stages

        goal_handle.succeed()
        return self._result('succeeded', '', 'Task completed')

    @staticmethod
    def _cancel_requested(goal_handle) -> bool:
        return goal_handle.is_cancel_requested

    def _cancel(self, goal_handle, route, finished_stages: int) -> ExecTask.Result:
        self._publish_feedback(
            goal_handle,
            'canceled',
            route,
            terminal_feedback(route, finished_stages),
        )
        goal_handle.canceled()
        return self._result('canceled', '', 'Task canceled')

    def _publish_feedback(self, goal_handle, state: str, route, step) -> None:
        feedback = ExecTask.Feedback()
        feedback.state = state
        feedback.progress = step.progress
        feedback.current_tag = step.current_tag
        feedback.next_tag = step.next_tag
        feedback.finished_stages = step.finished_stages
        feedback.route = route
        feedback.timestamp = self.get_clock().now().to_msg()
        goal_handle.publish_feedback(feedback)

    def _result(self, final_state: str, error_code: str, message: str) -> ExecTask.Result:
        result = ExecTask.Result()
        result.final_state = final_state
        result.error_code = error_code
        result.message = message
        result.finished_time = self.get_clock().now().to_msg()
        return result

    def destroy_node(self) -> None:
        self._action_server.destroy()
        super().destroy_node()


def main(args=None) -> None:
    rclpy.init(args=args)
    node = MockExecLayerNode()
    executor = MultiThreadedExecutor()
    executor.add_node(node)
    try:
        executor.spin()
    except KeyboardInterrupt:
        pass
    finally:
        executor.shutdown()
        node.destroy_node()
        rclpy.shutdown()
