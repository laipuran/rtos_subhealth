from __future__ import annotations

import queue
import threading
from typing import Optional

import rclpy
from rclpy.action import ActionClient
from rclpy.node import Node

from ros_interfaces.action import ExecTask
from ros_interfaces.msg import DiagnosisResult, VitalsStream
from std_msgs.msg import String

from .http_server import broadcast, broadcast_diagnosis, broadcast_vitals, init_app
from .diagnosis_store import DiagnosisRecord, DiagnosisStore
from .task_store import TaskRecord, TaskStore

_STATUS_SUCCEEDED = 4
_STATUS_CANCELED = 5
_STATUS_ABORTED = 6


def _metric_to_dict(m) -> dict:
    return {
        "data_src": m.data_src,
        "data_type": m.data_type,
        "latest": float(m.latest),
        "mean": float(m.mean),
        "min": float(m.min),
        "max": float(m.max),
        "trend": m.trend,
        "valid": bool(m.valid),
    }


class DescLayerNode(Node):
    def __init__(self) -> None:
        super().__init__("desc_layer_node")
        import os as _os
        db_dir = self.declare_parameter("db_dir", "").value
        if not db_dir:
            db_dir = _os.path.join(_os.getcwd(), "config")
        self._task_store = TaskStore(db_dir=db_dir)
        self._diagnosis_store = DiagnosisStore(db_dir=db_dir)
        self._goal_queue: queue.Queue[Optional[dict]] = queue.Queue()
        self._trigger_pub = self.create_publisher(String, "/diagnosis/trigger", 10)
        self._diagnosis_sub = self.create_subscription(
            DiagnosisResult, "/diagnosis/results", self._on_diagnosis, 10)
        self._vitals_sub = self.create_subscription(
            VitalsStream, "/diagnosis/monitor", self._on_vitals, 10)

        # RFC-009 §9.4: diagnosis retention / cleanup policy.
        self._diagnosis_retention_s = float(
            self.declare_parameter("diagnosis_retention_s", 0.0).value)
        cleanup_interval_s = float(
            self.declare_parameter("diagnosis_cleanup_interval_s", 3600.0).value)
        self.create_timer(cleanup_interval_s, self._cleanup_diagnoses)

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

        init_app(self._task_store, self._goal_queue, maps_dir, api_token,
                  diagnosis_store=self._diagnosis_store, trigger_pub=self._trigger_pub)
        port = self.declare_parameter("http_port", 5000).value
        t = threading.Thread(
            target=flask_app.run,
            kwargs={"host": "0.0.0.0", "port": port, "debug": False, "use_reloader": False},
            daemon=True,
        )
        t.start()
        self.get_logger().info(f"HTTP server started on 0.0.0.0:{port}, maps_dir={maps_dir}")

    def _on_diagnosis(self, msg: DiagnosisResult) -> None:
        metrics = [_metric_to_dict(m) for m in msg.metrics]
        rec = DiagnosisRecord(
            diagnosis_id=msg.diagnosis_id,
            source_ids=list(msg.source_ids),
            trigger_type=msg.trigger_type,
            severity=msg.severity,
            summary=msg.summary,
            possible_causes=list(msg.possible_causes),
            recommendations=list(msg.recommendations),
            confidence=float(msg.confidence),
            disclaimer=msg.disclaimer,
            raw_prompt=msg.raw_prompt,
            error_code=msg.error_code,
            error_message=msg.error_message,
            metrics=metrics,
        )
        self._diagnosis_store.add(rec)
        ts = msg.timestamp.sec + msg.timestamp.nanosec / 1e9
        broadcast_diagnosis({
            "diagnosis_id": msg.diagnosis_id,
            "trigger_type": msg.trigger_type,
            "severity": msg.severity,
            "summary": msg.summary,
            "possible_causes": list(msg.possible_causes),
            "recommendations": list(msg.recommendations),
            "confidence": float(msg.confidence),
            "timestamp": ts,
            "trace_id": msg.diagnosis_id,
            "error_code": msg.error_code,
            "error_message": msg.error_message,
            "metrics": metrics,
        })
        self.get_logger().info(
            f"diagnosis {msg.diagnosis_id} stored (severity={msg.severity}, "
            f"error={msg.error_code or '-'})")

    def _on_vitals(self, msg: VitalsStream) -> None:
        ts = msg.timestamp.sec + msg.timestamp.nanosec / 1e9
        broadcast_vitals({
            "timestamp": ts,
            "metrics": [_metric_to_dict(m) for m in msg.metrics],
        })

    def _cleanup_diagnoses(self) -> None:
        if self._diagnosis_retention_s <= 0:
            return
        deleted = self._diagnosis_store.purge_older_than(self._diagnosis_retention_s)
        if deleted:
            self.get_logger().info(f"purged {deleted} old diagnosis records")

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
            self.get_logger().error(f"{self._exec_action_name} action server not available")
            self._task_store.update(
                record.goal_id, state="failed", error_code="INTERNAL",
                message=f"{self._exec_action_name} action server unavailable",
            )
            broadcast(record.goal_id, "result",
                      {"final_state": "failed", "error_code": "INTERNAL"})
            return

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
            self._task_store.update(goal_id, state="failed", error_code="REJECTED",
                                     message="goal rejected by exec_layer")
            broadcast(goal_id, "result",
                      {"final_state": "failed", "error_code": "REJECTED"})
            return

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
        if getattr(fb, "route", None):
            updates["route"] = list(fb.route)
        if hasattr(fb, "finished_stages"):
            updates["finished_stages"] = int(fb.finished_stages)
        self._task_store.update(goal_id, **updates)
        ws_update = {
            "state": fb.state,
            "progress": fb.progress,
            "current_tag": fb.current_tag,
            "next_tag": fb.next_tag,
            "error_code": fb.error_code,
            "message": fb.message,
        }
        if getattr(fb, "route", None):
            ws_update["route"] = list(fb.route)
        if hasattr(fb, "finished_stages"):
            ws_update["finished_stages"] = int(fb.finished_stages)
        broadcast(goal_id, "feedback", ws_update)

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
