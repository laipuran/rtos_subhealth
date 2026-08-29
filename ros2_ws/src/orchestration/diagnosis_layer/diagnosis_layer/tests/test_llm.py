from __future__ import annotations

import pytest

from diagnosis_layer.llm_client import (
    LLMClient,
    build_messages,
    parse_diagnosis,
    passes_confidence,
)


def test_parse_diagnosis_valid():
    content = '{"severity":"mild","summary":"低血氧","possible_causes":["通气不足"],"recommendations":["吸氧"],"confidence":0.9,"disclaimer":"仅供参考"}'
    obj = parse_diagnosis(content)
    assert obj["severity"] == "mild"
    assert obj["confidence"] == 0.9
    assert obj["possible_causes"] == ["通气不足"]


def test_parse_diagnosis_with_fence():
    content = "```json\n" + '{"severity":"normal","summary":"s","possible_causes":[],"recommendations":[],"confidence":0.95,"disclaimer":"d"}' + "\n```"
    obj = parse_diagnosis(content)
    assert obj["severity"] == "normal"


def test_parse_diagnosis_invalid_severity():
    content = '{"severity":"weird","summary":"s","possible_causes":[],"recommendations":[],"confidence":0.9,"disclaimer":"d"}'
    with pytest.raises(ValueError):
        parse_diagnosis(content)


def test_parse_diagnosis_missing_field():
    content = '{"severity":"normal","summary":"s"}'
    with pytest.raises(ValueError):
        parse_diagnosis(content)


def test_passes_confidence():
    obj = {"confidence": 0.7}
    assert passes_confidence(obj, 0.8) is False
    assert passes_confidence({"confidence": 0.85}, 0.8) is True


def test_build_messages_injects_context():
    snap = {"trigger_type": "anomaly", "sources": [
        {"data_type": "spo2", "mean": 88.0, "min": 87.0, "max": 90.0, "latest": 88.0, "trend": "decreasing", "valid": True}]}
    system, user = build_messages(snap, "血氧低于90%为低血氧")
    assert "血氧低于90%为低血氧" in user
    assert "advisory" in system or "建议" in system


def test_llm_client_disabled():
    c = LLMClient()
    assert c.enabled is False
