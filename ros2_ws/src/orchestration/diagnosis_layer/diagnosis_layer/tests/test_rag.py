from __future__ import annotations

import os
import tempfile
import textwrap
from typing import Dict, List

from diagnosis_layer.rag import EmbeddingClient, Retriever, load_corpus


def _write_corpus(tmp: str) -> str:
    d = os.path.join(tmp, "medical")
    os.makedirs(d)
    with open(os.path.join(d, "spo2.md"), "w", encoding="utf-8") as fh:
        fh.write(textwrap.dedent("""
        # 血氧
        ## 低血氧
        血氧饱和度（SpO2）低于 90% 属于低血氧，可能由肺部疾病或通气不足引起。
        ## 正常
        正常成人静息血氧应在 95%-100%。
        """))
    with open(os.path.join(d, "hr.md"), "w", encoding="utf-8") as fh:
        fh.write(textwrap.dedent("""
        # 心率
        ## 心动过速
        心率高于 100 bpm 为心动过速。
        """))
    return d


def _snapshot() -> Dict:
    return {
        "sources": [{"data_type": "spo2", "mean": 88.0, "valid": True}],
        "trigger_type": "anomaly",
    }


def test_load_corpus_chunks():
    with tempfile.TemporaryDirectory() as tmp:
        d = _write_corpus(tmp)
        chunks = load_corpus(d)
        assert len(chunks) >= 2
        assert any("血氧" in c.text for c in chunks)


def test_keyword_retrieval():
    with tempfile.TemporaryDirectory() as tmp:
        d = _write_corpus(tmp)
        retriever = Retriever(corpus_dir=d, top_k=1)
        assert retriever.mode == "keyword"
        results = retriever.retrieve(_snapshot())
        assert results, "expected at least one retrieved chunk"
        assert "血氧" in results[0]


def test_empty_corpus_returns_empty():
    with tempfile.TemporaryDirectory() as tmp:
        retriever = Retriever(corpus_dir=tmp, top_k=3)
        assert retriever.retrieve(_snapshot()) == []


def test_embedding_retrieval_runs(monkeypatch):
    with tempfile.TemporaryDirectory() as tmp:
        d = _write_corpus(tmp)
        captured: Dict[str, List[str]] = {}

        def fake_embed(texts):
            captured["texts"] = texts
            return [[float(i == 0) for i in range(8)] for _ in texts]

        client = EmbeddingClient(base_url="http://example", api_key="k", model="m")
        monkeypatch.setattr(client, "embed", fake_embed)
        retriever = Retriever(corpus_dir=d, top_k=2, embedding=client)
        assert retriever.mode == "embeddings"
        res = retriever.retrieve(_snapshot())
        assert len(res) <= 2
        assert captured.get("texts")


def test_build_query_and_format_context():
    retriever = Retriever(corpus_dir=tempfile.mkdtemp(), top_k=3)
    query = retriever.build_query(_snapshot())
    assert "spo2" in query and "anomaly" in query
    ctx = Retriever.format_context(["a", "b"])
    assert "a" in ctx and "b" in ctx
