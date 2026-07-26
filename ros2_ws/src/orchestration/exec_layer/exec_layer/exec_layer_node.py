from __future__ import annotations

from typing import Optional

import rclpy
from rclpy.action import ActionServer, CancelResponse, GoalResponse
from rclpy.node import Node

from ros_interfaces.action import ExecTask
from ros_interfaces.msg import Segment
from ros_interfaces.srv import PlanPath

from .fsm import ExecFSM


class ExecLayerNode(Node):
    """执行层节点 —— 集成 FSM 的 RFC003/004 骨架实现

    三种任务类型的执行路径:
      patrol/navigate:  accepted → planning → 逐段 moving→...→stabilizing → completed
      hold:             accepted → holding → (等待取消/到期) → completed/canceled

    速率控制: 10 Hz 轮询，预留感知回调驱动接口.
    """

    def __init__(self) -> None:
        super().__init__("exec_layer_node")
        self._fsm = ExecFSM()

        self._planner_client = self.create_client(PlanPath, "plan_path")
        self._action_server = ActionServer(
            self,
            ExecTask,
            "exec_task",
            execute_callback=self.execute_task,
            goal_callback=self.handle_goal,
            cancel_callback=self.handle_cancel,
        )

        # 轮询定时器(可选): 用于 hold 超时 / 取消 / stopped 恢复等场景
        self._poll_timer = None
        self._goal_handle = None

    # ── Action 生命周期回调 ───────────────────────────────────

    def handle_goal(self, goal_request: ExecTask.Goal) -> GoalResponse:
        if not goal_request.type:
            self.get_logger().warn("Rejecting goal: missing type")
            return GoalResponse.REJECT
        return GoalResponse.ACCEPT

    def handle_cancel(self, goal_handle) -> CancelResponse:
        self._fsm.cancel()
        self.get_logger().info("Cancel accepted by state machine")
        return CancelResponse.ACCEPT

    def execute_task(self, goal_handle):
        self._goal_handle = goal_handle
        goal = goal_handle.request

        # ── 1. 接收任务 ────────────────────────────────────────
        self._fsm.accept_goal()
        self._publish_feedback(goal_handle)

        # ── 2. 按任务类型分流 ──────────────────────────────────
        if goal.type == "hold":
            return self._execute_hold(goal_handle, goal)

        return self._execute_motion(goal_handle, goal)

    # ── hold 任务 ────────────────────────────────────────────

    def _execute_hold(self, goal_handle, goal: ExecTask.Goal):
        """hold: 驻留等待，直到取消或 deadline 到达"""
        self._fsm.hold_position()
        self._publish_feedback(goal_handle)

        deadline_ns: int | None = None
        if goal.deadline_ms > 0:
            deadline_ns = goal.deadline_ms * 1_000_000  # ms → ns

        rate = self.create_rate(10)  # 10 Hz 轮询
        while rclpy.ok():
            if self._is_canceled(goal_handle):
                return self._make_result(goal_handle)

            # deadline 到达 → 正常结束
            if deadline_ns is not None:
                now_ns = self.get_clock().now().nanoseconds
                if now_ns >= deadline_ns:
                    self._fsm.hold_done()
                    self._publish_feedback(goal_handle)
                    return self._make_result(goal_handle)

            self._publish_feedback(goal_handle, progress=0.0)
            rate.sleep()

        # rclpy 关闭 → failed
        self._fsm.error_code = "INTERNAL"
        self._fsm.message = "node shutdown during hold"
        self._fsm.fail()
        return self._make_result(goal_handle)

    # ── patrol / navigate 任务 ──────────────────────────────

    def _execute_motion(self, goal_handle, goal: ExecTask.Goal):
        """patrol/navigate: 规划 → 逐段执行"""
        # 2a. 请求规划
        self._fsm.start_plan()
        self._publish_feedback(goal_handle)

        plan = self._request_plan(goal)
        if plan is None:
            if self._fsm.state != "canceled":
                self._fsm.error_code = "INTERNAL"
                self._fsm.message = "planner unavailable"
                self._fsm.plan_failed()
            return self._make_result(goal_handle)

        if plan.error_code not in ("OK", "PARTIAL"):
            self._fsm.error_code = plan.error_code
            self._fsm.message = plan.message or "planner returned error"
            self._fsm.plan_failed()
            return self._make_result(goal_handle)

        # 2b. 规划成功
        self._fsm.plan_success()
        self._publish_feedback(goal_handle, next_tag=plan.next_tag)

        segments = plan.segments
        total = len(segments)

        # 无路径段: 起点即终点，直接完成
        if total == 0:
            self._fsm.all_done()
            return self._make_result(goal_handle)

        # 2c. 逐段执行
        for i, segment in enumerate(segments):
            if self._is_canceled(goal_handle):
                return self._make_result(goal_handle)

            if i > 0:
                self._fsm.next_segment()

            self._execute_segment_phases(segment, goal_handle)
            if self._fsm.state == "canceled":
                return self._make_result(goal_handle)

            self._publish_feedback(
                goal_handle,
                progress=(i + 1) / total,
                current_tag=segment.to_tag,
                next_tag=segments[i + 1].to_tag
                if i + 1 < total
                else -1,
            )

        # 2d. 全部完成
        self._fsm.all_done()
        return self._make_result(goal_handle)

    # ── 单段执行 ──────────────────────────────────────────────

    def _execute_segment_phases(
        self, segment: Segment, goal_handle
    ) -> None:
        self._drive_phase("moving", segment)
        if self._is_canceled(goal_handle):
            return

        self._fsm.reach_tag()
        self._publish_feedback(goal_handle, current_tag=segment.to_tag)

        self._drive_phase("approaching", segment)
        if self._is_canceled(goal_handle):
            return

        self._fsm.rough_aligned()
        self._publish_feedback(goal_handle, current_tag=segment.to_tag)

        self._drive_phase("aligning", segment)
        if self._is_canceled(goal_handle):
            return

        self._fsm.aligned()
        self._publish_feedback(goal_handle, current_tag=segment.to_tag)

        self._drive_phase("stabilizing", segment)

    def _drive_phase(self, phase: str, segment: Segment) -> None:
        """骨架: 仅打日志. 替换为真实运动控制 + 感知回调驱动.

        真实实现应订阅 /perception/apriltag_detections,
        根据 phase 判断达标条件, 条件满足后调用 FSM 触发转移.
        """
        self.get_logger().info(
            f"  [{self._fsm.phase}] "
            f"tag={segment.to_tag}  from={segment.from_tag}"
        )

    # ── 规划器 ────────────────────────────────────────────────

    def _request_plan(self, goal: ExecTask.Goal) -> Optional[PlanPath.Response]:
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

    # ── 反馈 / 结果 ───────────────────────────────────────────

    def _publish_feedback(
        self,
        goal_handle,
        progress: float = 0.0,
        current_tag: int = -1,
        next_tag: int = -1,
    ) -> None:
        feedback = ExecTask.Feedback()
        feedback.state = self._fsm.feedback_state
        feedback.progress = progress
        feedback.current_tag = current_tag
        feedback.next_tag = next_tag
        feedback.error_code = self._fsm.error_code
        feedback.message = self._fsm.message
        feedback.timestamp = self.get_clock().now().to_msg()
        goal_handle.publish_feedback(feedback)

    def _is_canceled(self, goal_handle) -> bool:
        if goal_handle.is_cancel_requested and self._fsm.state != "canceled":
            self._fsm.cancel()
        return self._fsm.state == "canceled"

    def _make_result(self, goal_handle) -> ExecTask.Result:
        result = ExecTask.Result()
        result.final_state = self._fsm.final_state
        result.error_code = self._fsm.error_code
        result.message = self._fsm.message
        result.finished_time = self.get_clock().now().to_msg()

        if result.final_state == "succeeded":
            goal_handle.succeed()
        elif result.final_state == "failed":
            goal_handle.abort()
        elif result.final_state == "canceled":
            goal_handle.canceled()
        else:
            self.get_logger().warn(f"Unexpected final_state: {result.final_state}")
            goal_handle.abort()

        self._goal_handle = None
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
