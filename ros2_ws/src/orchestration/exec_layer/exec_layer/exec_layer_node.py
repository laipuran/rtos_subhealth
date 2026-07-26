from __future__ import annotations

import copy
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

from .fsm import ExecFSM
from .robot import RobotInterface, LowCmdRobot, SportClientRobot
from .detection_filter import DetectionFilter
from .move_stack import MoveStack
from .recovery import (
    RecoveryAction,
    RecoveryHandler,
    SegmentFailureAction,
)

_ARRIVAL_OFFSET_THRESHOLD = 0.05
_ALIGNING_OFFSET_THRESHOLD = 0.3
_ARRIVAL_DISTANCE_THRESHOLD = 500.0
_SEGMENT_DEADLINE_DEFAULT = 15.0



class ExecLayerNode(Node):
    def __init__(self) -> None:
        super().__init__("exec_layer_node")

        self._fsm = ExecFSM()
        self._cancel_requested = False
        self._robot: Optional[RobotInterface] = None
        self._detection_filter = DetectionFilter(window=10)
        self._move_stack = MoveStack()
        self._recovery_handler = RecoveryHandler()
        self._apriltag_sub = None
        self._blocked_tags: set[int] = set()
        self._plan_start_tag: int = -1
        self._goal_handle = None
        self._hold_rate = self.create_rate(10)

        robot_backend = self.declare_parameter("robot_backend", "mock").value
        dds_domain_id = self.declare_parameter("dds_domain_id", 1).value
        dds_interface = self.declare_parameter("dds_interface", "lo").value
        maps_dir = self.declare_parameter("maps_dir", "").value
        self._maps_dir = maps_dir

        if robot_backend == "sim":
            self._robot = LowCmdRobot(domain_id=dds_domain_id, interface=dds_interface)
            if not self._robot.connect():
                self.get_logger().error("LowCmdRobot connect failed")
        elif robot_backend == "real":
            self._robot = SportClientRobot(domain_id=dds_domain_id, interface=dds_interface)
            if not self._robot.connect():
                self.get_logger().error("SportClientRobot connect failed")
        else:
            self.get_logger().info("robot_backend=mock, using stub behavior")

        self._apriltag_sub = self.create_subscription(
            AprilTagDetections,
            "/perception/apriltag_detections",
            self._apriltag_callback,
            QoSProfile(depth=10, reliability=ReliabilityPolicy.BEST_EFFORT),
        )

        self._planner_client = self.create_client(PlanPath, "plan_path")

        self._action_server = ActionServer(
            self,
            ExecTask,
            "exec_task",
            execute_callback=self.execute_task,
            goal_callback=self.handle_goal,
            cancel_callback=self.handle_cancel,
        )

        if robot_backend == "sim" and isinstance(self._robot, LowCmdRobot):
            self._gait_timer = self.create_timer(0.002, self._gait_tick)
        else:
            self._gait_timer = None

        self._robot_backend = robot_backend
        self.get_logger().info(f"Exec Layer ready, backend={robot_backend}")

    def _gait_tick(self) -> None:
        if isinstance(self._robot, LowCmdRobot):
            self._robot._gait_tick()

    def handle_goal(self, goal_request: ExecTask.Goal) -> GoalResponse:
        if not goal_request.type:
            self.get_logger().warn("Rejecting goal: missing type")
            return GoalResponse.REJECT
        return GoalResponse.ACCEPT

    def handle_cancel(self, goal_handle) -> CancelResponse:
        self.get_logger().info("Cancel requested")
        self._cancel_requested = True
        self._fsm.cancel()
        if self._robot and self._robot.is_connected():
            self._robot.damp()
        return CancelResponse.ACCEPT

    def _apriltag_callback(self, msg: AprilTagDetections) -> None:
        self._detection_filter.update(msg)

    def execute_task(self, goal_handle):
        self._cancel_requested = False
        self._blocked_tags.clear()
        self._move_stack.clear()
        self._recovery_handler.clear()
        self._plan_start_tag = -1
        self._goal_handle = goal_handle

        goal = goal_handle.request
        self._fsm.accept_goal()
        self._publish_feedback(goal_handle)

        if goal.type == "hold":
            return self._execute_hold(goal_handle, goal)

        return self._execute_motion(goal_handle, goal)

    def _execute_hold(self, goal_handle, goal: ExecTask.Goal) -> ExecTask.Result:
        self._fsm.hold_position()
        self._publish_feedback(goal_handle)

        deadline_ns: int | None = None
        if goal.deadline_ms > 0:
            deadline_ns = goal.deadline_ms * 1_000_000

        while rclpy.ok():
            if self._is_canceled(goal_handle):
                return self._make_result(goal_handle)

            if deadline_ns is not None:
                now_ns = self.get_clock().now().nanoseconds
                if now_ns >= deadline_ns:
                    self._fsm.hold_done()
                    self._publish_feedback(goal_handle)
                    return self._make_result(goal_handle)

            self._publish_feedback(goal_handle, progress=0.0)
            self._hold_rate.sleep()

        self._fsm.error_code = "INTERNAL"
        self._fsm.message = "node shutdown during hold"
        self._fsm.fail()
        return self._make_result(goal_handle)

    def _execute_motion(self, goal_handle, goal: ExecTask.Goal) -> ExecTask.Result:
        self._fsm.start_plan()
        self._publish_feedback(goal_handle)

        self._plan_start_tag = self._detection_filter.get_most_frequent() or -1

        if self._is_canceled(goal_handle):
            return self._make_result(goal_handle)

        plan = self._request_plan(goal, self._plan_start_tag, self._blocked_tags)
        if plan is None:
            if self._robot_backend == "mock":
                self.get_logger().info("Mock mode: executing without planner")
                self._fsm.plan_success()
                self._execute_segments([], goal, goal_handle)
                self._fsm.all_done()
                return self._make_result(goal_handle)

            self._fsm.error_code = "INTERNAL"
            self._fsm.message = "planner unavailable"
            if not self._is_canceled(goal_handle):
                self._fsm.plan_failed()
            return self._make_result(goal_handle)

        if plan.error_code != "OK":
            self._fsm.error_code = plan.error_code
            self._fsm.message = plan.message or "planner returned error"
            if not self._is_canceled(goal_handle):
                self._fsm.plan_failed()
            return self._make_result(goal_handle)

        self._fsm.plan_success()
        self._publish_feedback(goal_handle, next_tag=plan.next_tag)

        self._execute_segments(list(plan.segments), goal, goal_handle)

        if self._fsm.is_terminal():
            return self._make_result(goal_handle)

        self._fsm.all_done()
        return self._make_result(goal_handle)

    def _request_plan(
        self, goal: ExecTask.Goal, start_tag: int,
        blocked: set[int],
    ) -> Optional[PlanPath.Response]:
        if not self._planner_client.wait_for_service(timeout_sec=1.0):
            self.get_logger().error("Planner service not available")
            return None

        request = PlanPath.Request()
        request.goal_id = ""
        request.task_type = goal.type
        request.route_id = goal.route_id
        request.target_tags = goal.target_tags
        request.start_tag = start_tag
        request.constraints = copy.copy(goal.constraints) if goal.constraints else None
        if request.constraints:
            request.constraints.avoid_tags = list(blocked)
        request.deadline_ms = goal.deadline_ms
        request.allow_partial = False
        request.replan_reason = ""

        future = self._planner_client.call_async(request)
        rclpy.spin_until_future_complete(self, future)
        if future.result() is None:
            self.get_logger().error("Planner call failed")
            return None
        return future.result()

    def _is_canceled(self, goal_handle) -> bool:
        if goal_handle.is_cancel_requested and self._fsm.state != "canceled":
            self._fsm.cancel()
        return self._fsm.state == "canceled"

    def _publish_feedback(
        self,
        goal_handle,
        progress: float = 0.0,
        current_tag: int = -1,
        next_tag: int = -1,
        finished_stages: int = 0,
        route: Optional[list[int]] = None,
    ) -> None:
        feedback = ExecTask.Feedback()
        feedback.state = self._fsm.feedback_state
        feedback.progress = progress
        feedback.current_tag = current_tag
        feedback.next_tag = next_tag
        feedback.finished_stages = finished_stages
        feedback.route = route or []
        feedback.error_code = self._fsm.error_code
        feedback.message = self._fsm.message
        feedback.timestamp = self.get_clock().now().to_msg()
        goal_handle.publish_feedback(feedback)

    def _build_route(self, segments: list[Segment]) -> list[int]:
        route: list[int] = []
        for s in segments:
            if not route or route[-1] != s.from_tag:
                route.append(s.from_tag)
            route.append(s.to_tag)
        return route

    def _execute_segments(
        self,
        segments: list[Segment],
        goal: ExecTask.Goal,
        goal_handle,
    ) -> None:
        idx = 0
        total = len(segments)
        route_tags = self._build_route(segments)
        while idx < total:
            if self._cancel_requested or self._is_canceled(goal_handle):
                return

            segment = segments[idx]
            self._publish_feedback(
                goal_handle,
                progress=idx / total if total > 0 else 1.0,
                current_tag=segment.from_tag,
                next_tag=segment.to_tag,
                finished_stages=idx,
                route=route_tags,
            )

            success = self._drive_segment(segment, goal)
            if success:
                idx += 1
                if idx < total:
                    self._fsm.next_segment()
                continue

            if self._cancel_requested or self._is_canceled(goal_handle):
                return

            action = self._recovery_handler.handle_segment_failure(segment)

            if action == SegmentFailureAction.RETRY:
                self.get_logger().info(
                    f"Retrying segment {segment.edge_id} "
                    f"(attempt {self._recovery_handler._retry_count.get(segment.edge_id, 0)})"
                )
                time.sleep(self._recovery_handler.get_retry_wait())
                continue

            # retries exhausted -> attempt reroute
            self._blocked_tags.add(segment.to_tag)

            current_pos = self._move_stack.current_position_tag() or -1
            preview = self._request_plan(
                goal, current_pos, self._blocked_tags,
            )
            if not preview or not preview.segments:
                self._fsm.error_code = "UNREACHABLE"
                self._fsm.message = f"no alternate route after tag {segment.to_tag}"
                self._fsm.fail()
                return

            self.get_logger().info(
                f"Rerouting around tag {segment.to_tag}, "
                f"alternate path possible, starting rollback"
            )

            self._fsm.request_replan()
            self._publish_feedback(goal_handle, route=route_tags)

            rollback = self._move_stack.get_rollback_path(self._blocked_tags)
            if rollback:
                self.get_logger().info(
                    f"Rolling back through {len(rollback)} segments"
                )
                for rb_seg in rollback:
                    if self._cancel_requested:
                        return
                    ok = self._drive_segment(rb_seg, goal)
                    if not ok:
                        return
                    self._move_stack.pop()

            current_pos = self._move_stack.current_position_tag() or -1
            self._publish_feedback(goal_handle, current_tag=current_pos, route=route_tags)

            new_plan = self._request_plan(
                goal, current_pos, self._blocked_tags,
            )
            if new_plan and new_plan.segments:
                self.get_logger().info(
                    f"New reroute plan: {len(new_plan.segments)} segments"
                )
                segments = list(new_plan.segments)
                route_tags = self._build_route(segments)
                idx = 0
                total = len(segments)
                self._recovery_handler.reset_retries(segment)
                self._fsm.replan_success()
                self._publish_feedback(
                    goal_handle, next_tag=new_plan.next_tag, route=route_tags,
                )
                continue

            self._fsm.error_code = "UNREACHABLE"
            self._fsm.message = (
                f"no alternate route after rollback from tag {segment.to_tag}"
            )
            self._fsm.replan_failed()
            return

        self._publish_feedback(
            goal_handle,
            progress=1.0,
            current_tag=segments[-1].to_tag if segments else -1,
            next_tag=-1,
            finished_stages=total,
            route=route_tags,
        )

    def _drive_segment(self, segment: Segment, goal: ExecTask.Goal) -> bool:
        to_tag = segment.to_tag
        self.get_logger().info(
            f"Driving segment {segment.from_tag} -> {to_tag} "
            f"(cost={segment.edge_cost})"
        )

        if not self._robot or not self._robot.is_connected():
            self.get_logger().info("No robot backend, simulating segment completion")
            time.sleep(1.0)
            self._move_stack.push(segment)
            self._move_stack.mark_completed(to_tag)
            return True

        self._robot.stand_up()
        time.sleep(0.5)

        dx, dy = self._compute_segment_direction(segment)
        if dx is not None and dy is not None:
            default_speed = 0.3
            speed = (
                goal.constraints.max_speed_mps
                if goal.constraints and goal.constraints.max_speed_mps > 0.0
                else default_speed
            )
            total_dist = (dx ** 2 + dy ** 2) ** 0.5
            if total_dist > 0.001:
                vx = (dx / total_dist) * min(speed, default_speed)
                vy = (dy / total_dist) * min(speed, default_speed)
            else:
                vx, vy = 0.3, 0.0
        else:
            vx, vy = 0.3, 0.0

        self._robot.move(vx, vy, 0.0)
        self._move_stack.push(segment)

        deadline = (
            goal.deadline_ms / 1000.0
            if goal.deadline_ms > 0
            else _SEGMENT_DEADLINE_DEFAULT
        )
        if dx is not None and dy is not None and deadline == _SEGMENT_DEADLINE_DEFAULT:
            dist = (dx ** 2 + dy ** 2) ** 0.5
            speed_val = goal.constraints.max_speed_mps if (
                goal.constraints and goal.constraints.max_speed_mps > 0.0
            ) else 0.3
            estimated_time = dist / max(speed_val, 0.05)
            deadline = max(deadline, estimated_time * 1.5 + 5.0)

        start_time = time.time()
        reach_triggered = False
        align_triggered = False
        stable_triggered = False

        while time.time() - start_time < deadline:
            if self._cancel_requested:
                self._robot.damp()
                self._move_stack.pop()
                return False

            if not self._detection_filter.is_stable(to_tag, min_frames=3):
                action = self._recovery_handler.handle_lost_tag()
                if action == RecoveryAction.CONTINUE:
                    speed_scale = 0.5
                    self._robot.move(vx * speed_scale, vy * speed_scale, 0.0)
                    time.sleep(0.1)
                    continue
                elif action == RecoveryAction.SEARCH:
                    self._robot.damp()
                    self.get_logger().info(f"Searching for tag {to_tag}...")
                    self._rotate_search(to_tag)
                    if self._cancel_requested:
                        self._move_stack.pop()
                        return False
                    if self._detection_filter.is_stable(to_tag, min_frames=3):
                        self._recovery_handler.on_tag_found()
                        self._robot.move(vx, vy, 0.0)
                        continue
                    else:
                        self.get_logger().warn(f"Tag {to_tag} lost, failing segment")
                        self._robot.damp()
                        self._move_stack.pop()
                        return False
                else:
                    self.get_logger().warn(f"Tag {to_tag} lost, failing segment")
                    self._robot.damp()
                    self._move_stack.pop()
                    return False
            else:
                self._recovery_handler.on_tag_found()
                if not reach_triggered:
                    self._fsm.reach_tag()
                    reach_triggered = True

            best = self._detection_filter.get_best(to_tag)
            if best is not None:
                offset_x = abs(best.center_offset_x)
                offset_y = abs(best.center_offset_y)

                if not align_triggered and offset_x < _ALIGNING_OFFSET_THRESHOLD and offset_y < _ALIGNING_OFFSET_THRESHOLD:
                    self._fsm.rough_aligned()
                    align_triggered = True

                if not stable_triggered and offset_x < _ARRIVAL_OFFSET_THRESHOLD and offset_y < _ARRIVAL_OFFSET_THRESHOLD:
                    self._fsm.aligned()
                    stable_triggered = True

                if (offset_x < _ARRIVAL_OFFSET_THRESHOLD
                        and offset_y < _ARRIVAL_OFFSET_THRESHOLD
                        and best.distance < _ARRIVAL_DISTANCE_THRESHOLD):
                    self.get_logger().info(
                        f"Reached tag {to_tag}, distance={best.distance:.0f}mm"
                    )
                    self._robot.damp()
                    self._move_stack.mark_completed(to_tag)
                    self._recovery_handler.reset_retries(segment)
                    return True

            time.sleep(0.1)

        self.get_logger().warn(f"Timeout reaching tag {to_tag}")
        self._robot.damp()
        self._move_stack.pop()
        return False

    def _compute_segment_direction(
        self, segment: Segment,
    ) -> tuple[Optional[float], Optional[float]]:
        from .planner.graph import TagGraph
        if not self._maps_dir:
            return None, None
        import os
        filepath = os.path.join(self._maps_dir, "default.json")
        try:
            graph = TagGraph.load_json(filepath)
        except Exception:
            return None, None

        from_tag = graph.tags.get(segment.from_tag)
        to_tag = graph.tags.get(segment.to_tag)
        if from_tag is None or to_tag is None:
            return None, None

        dx = to_tag.x - from_tag.x
        dy = to_tag.y - from_tag.y
        return dx, dy

    def _rotate_search(self, target_id: int) -> None:
        if not self._robot or not self._robot.is_connected():
            return
        for _ in range(3):
            if self._cancel_requested:
                return
            self._robot.move(0.0, 0.0, 0.5)
            time.sleep(1.0)
            if self._detection_filter.is_stable(target_id, min_frames=2):
                self._robot.damp()
                return
        self._robot.damp()

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
        if node._robot and node._robot.is_connected():
            node._robot.damp()
            node._robot.disconnect()
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
