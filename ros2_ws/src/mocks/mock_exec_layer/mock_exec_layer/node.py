"""ROS 2 action server for deterministic task execution."""

import json
import time

import rclpy
from rclpy.action import ActionServer, CancelResponse, GoalResponse
from rclpy.callback_groups import ReentrantCallbackGroup
from rclpy.executors import MultiThreadedExecutor
from rclpy.node import Node
from task_interfaces.action import ExecuteTask

from .contract import (
    InvalidPayload,
    parse_payload,
    UnsupportedPrimitive,
)
from .execution import (
    canceled_feedback,
    execution_steps,
    validate_step_delay,
)
from .terminal_state import GoalTerminalCoordinator


class MockExecLayerNode(Node):
    """Serve predictable execution results for integration testing."""

    def __init__(self) -> None:
        super().__init__('mock_exec_layer')
        self.declare_parameter('action_name', '/mock_exec/execute_task')
        self.declare_parameter('device_id', 'mock_exec')
        self.declare_parameter('step_delay_s', 1.0)
        self.declare_parameter('fail_target_tag', 2**31)

        action_name = self.get_parameter('action_name').value
        self._device_id = str(self.get_parameter('device_id').value)
        self._step_delay_s = validate_step_delay(
            float(self.get_parameter('step_delay_s').value)
        )
        self._fail_target_tag = int(self.get_parameter('fail_target_tag').value)
        self._terminal_states = GoalTerminalCoordinator()
        self._action_server = ActionServer(
            self,
            ExecuteTask,
            action_name,
            execute_callback=self.execute_callback,
            goal_callback=self.goal_callback,
            cancel_callback=self.cancel_callback,
            callback_group=ReentrantCallbackGroup(),
        )

    def goal_callback(self, goal_request: ExecuteTask.Goal) -> GoalResponse:
        if not goal_request.task_id or goal_request.device_id != self._device_id:
            return GoalResponse.REJECT
        if goal_request.deadline_unix_ms < 0:
            return GoalResponse.REJECT
        try:
            parse_payload(goal_request.primitive, goal_request.payload_json)
        except (InvalidPayload, UnsupportedPrimitive):
            return GoalResponse.REJECT
        return GoalResponse.ACCEPT

    def cancel_callback(self, goal_handle) -> CancelResponse:
        if self._terminal_states.request_cancel(goal_handle):
            return CancelResponse.ACCEPT
        return CancelResponse.REJECT

    def execute_callback(self, goal_handle) -> ExecuteTask.Result:
        request = goal_handle.request
        try:
            payload = parse_payload(request.primitive, request.payload_json)
            steps = execution_steps(request.primitive, payload)
        except InvalidPayload as error:
            return self._abort_or_cancel(
                goal_handle, None, 'INVALID_PAYLOAD', str(error)
            )
        except UnsupportedPrimitive as error:
            return self._abort_or_cancel(
                goal_handle, None, 'UNSUPPORTED_PRIMITIVE', str(error)
            )

        last_step = None
        for step in steps:
            if self._step_delay_s > 0.0:
                time.sleep(self._step_delay_s)
            if self._terminal_states.is_cancel_pending(goal_handle):
                return self._cancel(goal_handle, last_step)
            if self._deadline_expired(request.deadline_unix_ms):
                return self._abort_or_cancel(
                    goal_handle,
                    last_step,
                    'DEADLINE_EXCEEDED',
                    'Task deadline elapsed',
                )

            self._publish_feedback(goal_handle, 'running', step)
            last_step = step

            if (
                request.primitive == 'go_to_tag'
                and payload['target_tag'] == self._fail_target_tag
            ):
                return self._abort_or_cancel(
                    goal_handle, last_step, 'SDK_ERROR', 'Configured mock failure'
                )

        if self._terminal_states.commit_success(goal_handle):
            goal_handle.succeed()
            return self._result(goal_handle, 'succeeded', '', 'Task completed')
        return self._cancel(goal_handle, last_step)

    def _cancel(self, goal_handle, last_step) -> ExecuteTask.Result:
        while not goal_handle.is_cancel_requested:
            time.sleep(0.001)
        if not self._terminal_states.commit_canceled(goal_handle):
            raise RuntimeError('cancellation was not accepted for this goal')
        self._publish_feedback(
            goal_handle, 'canceled', canceled_feedback(last_step)
        )
        goal_handle.canceled()
        return self._result(goal_handle, 'canceled', '', 'Task canceled')

    def _abort_or_cancel(
        self, goal_handle, last_step, error_code: str, message: str
    ) -> ExecuteTask.Result:
        if self._terminal_states.commit_abort(goal_handle):
            goal_handle.abort()
            return self._result(goal_handle, 'failed', error_code, message)
        return self._cancel(goal_handle, last_step)

    def _publish_feedback(self, goal_handle, state: str, step) -> None:
        feedback = ExecuteTask.Feedback()
        feedback.task_id = goal_handle.request.task_id
        feedback.state = state
        feedback.progress = max(0.0, min(1.0, step.progress))
        feedback.phase = step.phase
        feedback.details_json = json.dumps(
            step.details, separators=(',', ':'), sort_keys=True
        )
        feedback.timestamp = self.get_clock().now().to_msg()
        goal_handle.publish_feedback(feedback)

    def _result(
        self, goal_handle, final_state: str, error_code: str, message: str
    ) -> ExecuteTask.Result:
        result = ExecuteTask.Result()
        result.task_id = goal_handle.request.task_id
        result.final_state = final_state
        result.error_code = error_code
        result.message = message
        result.finished_time = self.get_clock().now().to_msg()
        return result

    def _deadline_expired(self, deadline_unix_ms: int) -> bool:
        if deadline_unix_ms == 0:
            return False
        return time.time_ns() // 1_000_000 >= deadline_unix_ms

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
