from __future__ import annotations

import threading
import uuid
from typing import Dict, List, Optional, Set, Tuple

import rclpy
from rclpy.node import Node
from rclpy.qos import QoSProfile, ReliabilityPolicy, HistoryPolicy
from std_msgs.msg import String

from physio_interfaces.msg import PhysioSample
from ros_interfaces.msg import DiagnosisResult

from .aggregator import Sample, Window, build_snapshot, is_anomalous, parse_thresholds
from .llm_client import LLMClient, build_messages, parse_diagnosis, passes_confidence
from .rag import EmbeddingClient, Retriever

_DATA_TYPE_BY_SRC = {
    "mock_spo2": "spo2",
    "mock_heart_rate": "heart_rate",
    "mock_bp_systolic": "systolic_mmhg",
    "mock_bp_diastolic": "diastolic_mmhg",
    "mock_body_temp": "body_temp_c",
    "mock_respiratory_rate": "respiratory_rate",
}


def _to_sec(msg_time) -> float:
    return float(msg_time.sec) + float(msg_time.nanosec) / 1e9


def _job_key(trigger_type: str, source_ids: List[str]) -> Tuple[str, frozenset]:
    return (trigger_type, frozenset(source_ids))


class DiagnosisLayerNode(Node):
    def __init__(self) -> None:
        super().__init__("diagnosis_layer")

        data_sources = self.declare_parameter(
            "data_sources",
            list(_DATA_TYPE_BY_SRC.keys()),
        ).value
        self._window_seconds = float(self.declare_parameter("window_seconds", 60.0).value)
        self._periodic_interval_s = float(self.declare_parameter("periodic_interval_s", 60.0).value)
        self._confidence_min = float(self.declare_parameter("confidence_min", 0.8).value)
        self._anomaly_cooldown_s = float(self.declare_parameter("anomaly_cooldown_s", 30.0).value)
        # RFC-009 §9.3: configurable per-data_type anomaly thresholds (JSON string).
        thresholds_json = self.declare_parameter("anomaly_thresholds", "").value
        self._thresholds = parse_thresholds(thresholds_json or "")

        medical_dir = self.declare_parameter("medical_corpus_dir", "").value
        top_k = int(self.declare_parameter("rag_top_k", 3).value)
        emb_base = self.declare_parameter("embedding_base_url", "").value
        emb_key = self.declare_parameter("embedding_api_key", "").value
        emb_model = self.declare_parameter("embedding_model", "").value
        embedding = EmbeddingClient(emb_base, emb_key, emb_model) if emb_base else None
        self._retriever = Retriever(
            corpus_dir=medical_dir or None, top_k=top_k, embedding=embedding)

        llm_base = self.declare_parameter("llm_base_url", "").value
        llm_key = self.declare_parameter("llm_api_key", "").value
        llm_model = self.declare_parameter("llm_model", "gpt-4o-mini").value
        self._llm = LLMClient(llm_base, llm_key, llm_model)

        self._windows: Dict[str, Window] = {
            ds: Window(ds, _DATA_TYPE_BY_SRC.get(ds, ds), self._window_seconds)
            for ds in data_sources
        }
        self._anomaly_state: Dict[str, bool] = {ds: False for ds in data_sources}
        self._anomaly_last_trigger: Dict[str, float] = {ds: 0.0 for ds in data_sources}
        self._running: Dict[Tuple, bool] = {}
        self._anomaly_running = 0
        self._lock = threading.Lock()

        qos = QoSProfile(reliability=ReliabilityPolicy.RELIABLE,
                         history=HistoryPolicy.KEEP_LAST, depth=10)
        for ds in data_sources:
            self.create_subscription(
                PhysioSample, f"/physio/{ds}",
                lambda msg, ds=ds: self._on_sample(ds, msg), qos)
        self.create_subscription(String, "/diagnosis/trigger", self._on_manual_trigger, qos)
        self._result_pub = self.create_publisher(DiagnosisResult, "/diagnosis/results", qos)

        self._timer = self.create_timer(self._periodic_interval_s, self._on_periodic)
        self.get_logger().info(
            f"Diagnosis layer ready. RAG mode={self._retriever.mode}, "
            f"LLM enabled={self._llm.enabled}, sources={list(data_sources)}")

    # --- sample ingestion + anomaly detection ---

    def _on_sample(self, data_src: str, msg: PhysioSample) -> None:
        win = self._windows.get(data_src)
        if win is None:
            return
        now = self.get_clock().now().nanoseconds / 1e9
        win.prune(now)
        win.add(Sample(t=_to_sec(msg.timestamp), value=msg.data, valid=msg.valid))

        anomalous = (not msg.valid) or is_anomalous(msg.data_type, msg.data, self._thresholds)
        was = self._anomaly_state.get(data_src, False)
        if anomalous and not was:
            # transition into anomaly -> trigger once (RFC-009: 恢复后不重复触发)
            self._anomaly_state[data_src] = True
            if now - self._anomaly_last_trigger.get(data_src, 0.0) >= self._anomaly_cooldown_s:
                self._anomaly_last_trigger[data_src] = now
                self._start_job("anomaly", [data_src])
        elif not anomalous and was:
            self._anomaly_state[data_src] = False

    def _on_manual_trigger(self, msg: String) -> None:
        payload = (msg.data or "manual").strip().lower()
        force_id = ""
        if ":" in payload:
            kind, force_id = payload.split(":", 1)
        else:
            kind = payload
        if kind in ("manual", ""):
            # RFC-005 §5.3.4: carry the originating request trace_id through as
            # the diagnosis_id so it surfaces in the WS event's trace_id field.
            self._start_job("manual", list(self._windows.keys()), force_id=force_id or None)

    def _on_periodic(self) -> None:
        self._start_job("periodic", list(self._windows.keys()))

    # --- concurrency / job control ---

    def _start_job(self, trigger_type: str, source_ids: List[str],
                   force_id: Optional[str] = None) -> None:
        if not source_ids:
            return
        key = _job_key(trigger_type, source_ids)
        with self._lock:
            if key in self._running:
                self.get_logger().debug(f"skip duplicate job {key}")
                return
            self._running[key] = True
            if trigger_type == "anomaly":
                self._anomaly_running += 1
        self.get_logger().info(f"start diagnosis job {key} force_id={force_id}")
        threading.Thread(
            target=self._run_job, args=(trigger_type, source_ids, key, force_id),
            daemon=True).start()

    def _finish_job(self, key: Tuple) -> None:
        with self._lock:
            self._running.pop(key, None)
            if key[0] == "anomaly":
                self._anomaly_running = max(0, self._anomaly_running - 1)

    # --- diagnosis pipeline ---

    def _run_job(self, trigger_type: str, source_ids: List[str], key: Tuple,
                 force_id: Optional[str] = None) -> None:
        try:
            now = self.get_clock().now().nanoseconds / 1e9
            windows = {ds: self._windows[ds] for ds in source_ids if ds in self._windows}
            for w in windows.values():
                w.prune(now)
            snapshot = build_snapshot(windows, trigger_type)

            # RAG retrieval (skipped for non-anomaly when anomaly job running -> 资源降级)
            context = ""
            with self._lock:
                anomaly_busy = self._anomaly_running > 0 and trigger_type != "anomaly"
            if not anomaly_busy:
                context = self._retriever.format_context(self._retriever.retrieve(snapshot))

            self._diagnose_and_publish(trigger_type, source_ids, snapshot, context, force_id)
        finally:
            self._finish_job(key)

    def _diagnose_and_publish(self, trigger_type, source_ids, snapshot, context,
                              force_id: Optional[str] = None) -> None:
        system, user = build_messages(snapshot, context)
        diagnosis_id = force_id or uuid.uuid4().hex[:16]
        raw_prompt = json_dumps({"system": system, "user": user})

        result = DiagnosisResult()
        result.diagnosis_id = diagnosis_id
        result.source_ids = source_ids
        result.trigger_type = trigger_type
        result.raw_prompt = raw_prompt
        result.timestamp = self.get_clock().now().to_msg()

        if not self._llm.enabled:
            result.error_code = "LLM_DISABLED"
            result.error_message = "LLM not configured; RAG context prepared"
            result.summary = context[:500]
            self._result_pub.publish(result)
            return

        content = None
        last_err = ""
        for _ in range(3):  # RFC-009: 最多 3 次（含首次）
            try:
                content = self._llm.complete(system, user)
                obj = parse_diagnosis(content)
            except Exception as exc:  # noqa: BLE001
                last_err = str(exc)
                self.get_logger().warning(f"LLM attempt failed: {last_err}")
                continue
            if not passes_confidence(obj, self._confidence_min):
                result.error_code = "LOW_CONFIDENCE"
                result.error_message = f"confidence {obj.get('confidence')} < {self._confidence_min}"
                self._result_pub.publish(result)
                return
            self._fill_result(result, obj)
            self._result_pub.publish(result)
            return

        result.error_code = "LLM_PARSE_FAILED"
        result.error_message = last_err or "all attempts failed"
        self._result_pub.publish(result)

    def _fill_result(self, result: DiagnosisResult, obj: dict) -> None:
        result.severity = obj.get("severity", "normal")
        result.summary = obj.get("summary", "")
        result.possible_causes = list(obj.get("possible_causes", []))
        result.recommendations = list(obj.get("recommendations", []))
        result.confidence = float(obj.get("confidence", 0.0))
        result.disclaimer = obj.get("disclaimer", "")


def json_dumps(obj) -> str:
    import json
    return json.dumps(obj, ensure_ascii=False)


def main() -> None:
    rclpy.init()
    node = DiagnosisLayerNode()
    try:
        rclpy.spin(node)
    except KeyboardInterrupt:
        pass
    finally:
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
