"""Headless demo: 不依赖 ROS / LLM / 真实传感器，演示诊断链路核心逻辑。

运行:
  cd ros2_ws/src/orchestration/diagnosis_layer
  $env:PYTHONPATH = "$PWD;$env:PYTHONPATH"
  python -m diagnosis_layer.demo
"""
from __future__ import annotations

from diagnosis_layer.aggregator import Sample, Window, build_snapshot, is_anomalous
from diagnosis_layer.rag import Retriever
from diagnosis_layer.llm_client import build_messages


def main() -> None:
    # 1) 模拟传感器：构造一个含异常 spo2 的窗口
    win = Window("mock_spo2", "spo2", window_seconds=60.0)
    t0 = 1_000.0
    for i, v in enumerate([96.0, 94.0, 89.0, 86.0, 85.0]):
        win.add(Sample(t=t0 + i * 5, value=v, valid=True))
    win.prune(t0 + 20)

    # 2) 异常检测（规则）
    latest = win.samples[-1]
    print(f"[anomaly] spo2={latest.value} -> anomalous={is_anomalous('spo2', latest.value)}\n")

    # 3) 构造多源快照
    snapshot = build_snapshot({"mock_spo2": win}, trigger_type="anomaly")
    print("[snapshot]", snapshot, "\n")

    # 4) RAG 检索（关键词模式，无需 embedding）
    retriever = Retriever(top_k=2)
    print(f"[rag] mode={retriever.mode}, chunks={retriever.chunk_count}")
    context = retriever.format_context(retriever.retrieve(snapshot))
    print("[rag] retrieved context:\n", context, "\n")

    # 5) 拼装发给 LLM 的 prompt（LLM 未配置时，这段就是节点会保存的 raw_prompt）
    system, user = build_messages(snapshot, context)
    print("[prompt.user]\n", user, "\n")

    # 6) LLM_DISABLED 时节点会发布的结果（保留 RAG 上下文）
    print("[result-without-llm] error_code=LLM_DISABLED, summary=context[:500]")
    print("=> 链路已跑通：采样→聚合→异常→RAG检索→prompt 就绪，仅差真正调用 LLM 生成 JSON。")


if __name__ == "__main__":
    main()
