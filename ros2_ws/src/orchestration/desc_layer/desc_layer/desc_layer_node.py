from __future__ import annotations

import queue
import threading
from typing import Optional

import rclpy
from rclpy.action import ActionClient
from rclpy.node import Node

from ros_interfaces.action import ExecTask

from .http_server import broadcast, init_app
from .task_store import TaskRecord, TaskStore
from .log_util import info as log_info, error as log_error

_STATUS_SUCCEEDED = 4
_STATUS_CANCELED = 5
_STATUS_ABORTED = 6


class DescLayerNode(Node):
    def __init__(self) -> None:
        super().__init__("desc_layer_node")
        import os as _os
        db_dir = self.declare_parameter("db_dir", "").value
        if not db_dir:
            db_dir = _os.path.join(_os.getcwd(), "config")
        self._task_store = TaskStore(db_dir=db_dir)
        self._goal_queue: queue.Queue[Optional[dict]] = queue.Queue()
        self._goal_handles: dict[str, "GoalHandle"] = {}

        exec_action_name = self.declare_parameter("exec_action_name", "exec_task").value
        self._action_client = ActionClient(self, ExecTask, exec_action_name)
        self._exec_action_name = exec_action_name

        self._timer = self.create_timer(0.1, self._process_queue)

        self._start_http_server()

        self.get_logger().info(f"Desc Layer ready, connecting to action: {exec_action_name}")

    def _start_http_server(self) -> None:
        from .http_server import app as flask_app

        maps_dir = self.declare_parameter("maps_dir", "").value
        if not maps_dir:
            import os
            maps_dir = os.path.join(os.getcwd(), "config", "maps")
            if not os.path.isdir(maps_dir):
                self.get_logger().warn(f"maps_dir not found at {maps_dir}, trying fallback")
                maps_dir = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", "..", "config", "maps")
                maps_dir = os.path.normpath(maps_dir)

        api_token = self.declare_parameter("api_token", "").value
        if api_token:
            self.get_logger().info("API token authentication enabled")

        init_app(self._task_store, self._goal_queue, maps_dir, api_token)
        port = self.declare_parameter("http_port", 5000).value
        t = threading.Thread(
            target=flask_app.run,
            kwargs={"host": "0.0.0.0", "port": port, "debug": False, "use_reloader": False},
            daemon=True,
        )
        t.start()
        self.get_logger().info(f"HTTP server started on 0.0.0.0:{port}, maps_dir={maps_dir}")

    def _process_queue(self) -> None:
        try:
            msg = self._goal_queue.get_nowait()
        except queue.Empty:
            return

        if msg is None:
            return

        goal_id = msg.get("goal_id", "")

        if msg.get("cancel"):
            self._handle_cancel(goal_id)
            return

        record = self._task_store.get(goal_id)
        if record is None:
            self.get_logger().warn(f"task {goal_id} not found in store")
            return

        self._send_goal(record)

    def _send_goal(self, record: TaskRecord) -> None:
        if not self._action_client.wait_for_server(timeout_sec=5.0):
            log_error("desc", "ACT", f"{self._exec_action_name} unavailable",
                      task=record.goal_id[:8])
            self._task_store.update(
                record.goal_id, state="failed", error_code="INTERNAL",
                message=f"{self._exec_action_name} action server unavailable",
            )
            broadcast(record.goal_id, "result",
                      {"final_state": "failed", "error_code": "INTERNAL"})
            return

        log_info("desc", "ACT", f"send_goal → {self._exec_action_name}",
                 task=record.goal_id[:8], type=record.goal.type)

        goal = ExecTask.Goal()
        goal.type = record.goal.type
        goal.priority = record.goal.priority
        goal.route_id = record.goal.route_id
        goal.target_tags = record.goal.target_tags
        goal.constraints = record.goal.constraints
        goal.deadline_ms = record.goal.deadline_ms

        send_goal_future = self._action_client.send_goal_async(
            goal,
            feedback_callback=lambda fb: self._feedback_callback(record.goal_id, fb),
        )
        send_goal_future.add_done_callback(
            lambda f: self._goal_response_callback(record.goal_id, f)
        )

    def _goal_response_callback(self, goal_id: str, future) -> None:
        goal_handle = future.result()
        if not goal_handle or not goal_handle.accepted:
            log_error("desc", "ACT", f"goal rejected", task=goal_id[:8])
            self._task_store.update(goal_id, state="failed", error_code="REJECTED",
                                     message="goal rejected by exec_layer")
            broadcast(goal_id, "result",
                      {"final_state": "failed", "error_code": "REJECTED"})
            return

        log_info("desc", "ACT", f"accepted → running", task=goal_id[:8])
        self._goal_handles[goal_id] = goal_handle
        self._task_store.update(goal_id, state="running")

        get_result_future = goal_handle.get_result_async()
        get_result_future.add_done_callback(
            lambda f: self._result_callback(goal_id, f)
        )

    def _feedback_callback(self, goal_id: str, feedback_msg) -> None:
        fb = feedback_msg.feedback
        updates = {
            "state": fb.state,
            "progress": fb.progress,
            "current_tag": fb.current_tag,
            "next_tag": fb.next_tag,
        }
        if fb.error_code:
            updates["error_code"] = fb.error_code
        if fb.message:
            updates["message"] = fb.message
        self._task_store.update(goal_id, **updates)
        broadcast(
            goal_id,
            "feedback",
            {
                "state": fb.state,
                "progress": fb.progress,
                "current_tag": fb.current_tag,
                "next_tag": fb.next_tag,
                "error_code": fb.error_code,
                "message": fb.message,
            },
        )

    def _result_callback(self, goal_id: str, future) -> None:
        response = future.result()
        result = response.result
        status = response.status

        if status == _STATUS_SUCCEEDED:
            final_state = "succeeded"
        elif status == _STATUS_CANCELED:
            final_state = "canceled"
        elif status == _STATUS_ABORTED:
            final_state = "failed"
        else:
            final_state = "unknown"

        log_info("desc", "ACT", f"result: {final_state}", task=goal_id[:8],
                 err=result.error_code if result else "")

        self._task_store.update(
            goal_id,
            state=final_state,
            error_code=result.error_code if result else "",
            message=result.message if result else "",
            result=result,
        )
        self._goal_handles.pop(goal_id, None)
        broadcast(
            goal_id,
            "result",
            {
                "final_state": final_state,
                "error_code": result.error_code if result else "",
                "message": result.message if result else "",
            },
        )

    def _handle_cancel(self, goal_id: str) -> None:
        gh = self._goal_handles.get(goal_id)
        if gh is not None:
            cancel_future = gh.cancel_goal_async()
            cancel_future.add_done_callback(lambda _: self.get_logger().info(
                f"cancel sent for {goal_id}"))
        else:
            self._task_store.update(goal_id, state="canceled",
                                     message="cancel requested, no active goal handle")
            broadcast(goal_id, "result",
                      {"final_state": "canceled", "message": "no active goal handle"})


def main() -> None:
    rclpy.init()
    node = DescLayerNode()
    try:
        rclpy.spin(node)
    except KeyboardInterrupt:
        node._goal_queue.put(None)
    finally:
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
