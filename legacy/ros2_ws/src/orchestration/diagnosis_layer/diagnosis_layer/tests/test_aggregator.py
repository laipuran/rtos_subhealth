from __future__ import annotations

from diagnosis_layer.aggregator import (
    ANOMALY_THRESHOLDS,
    Sample,
    Window,
    build_snapshot,
    is_anomalous,
    parse_thresholds,
)


def _win(data_type="spo2", values=(88.0, 89.0, 87.0, 86.0)):
    w = Window("mock_spo2", data_type)
    t0 = 1000.0
    for i, v in enumerate(values):
        w.add(Sample(t=t0 + i, value=v, valid=True))
    return w


def test_window_stats():
    w = _win()
    st = w.stats()
    assert st["mean"] == 87.5
    assert st["min"] == 86.0
    assert st["max"] == 89.0
    assert st["valid"] is True
    assert st["trend"] in ("decreasing", "stable", "increasing", "unknown")


def test_window_prune():
    w = Window("mock_spo2", "spo2", window_seconds=10.0)
    w.add(Sample(t=0.0, value=90.0))
    w.add(Sample(t=5.0, value=91.0))
    w.add(Sample(t=20.0, value=92.0))
    w.prune(25.0)
    assert len(w.samples) == 1  # t=0 and t=5 pruned (window 10s vs now 25)


def test_is_anomalous():
    assert is_anomalous("spo2", 85.0) is True
    assert is_anomalous("spo2", 97.0) is False
    assert is_anomalous("heart_rate", 110.0) is True
    assert is_anomalous("systolic_mmhg", 145.0) is True
    assert is_anomalous("body_temp_c", 38.0) is True
    assert is_anomalous("respiratory_rate", 22.0) is True
    assert is_anomalous("respiratory_rate", 16.0) is False


def test_build_snapshot():
    windows = {"mock_spo2": _win(), "mock_heart_rate": _win("heart_rate", (70, 72, 71, 69))}
    snap = build_snapshot(windows, "periodic")
    assert snap["trigger_type"] == "periodic"
    assert len(snap["sources"]) == 2
    types = {s["data_type"] for s in snap["sources"]}
    assert types == {"spo2", "heart_rate"}


def test_parse_thresholds_default_and_custom():
    default = parse_thresholds("")
    assert default["spo2"]["low"] == 90.0
    custom = parse_thresholds('{"spo2": {"low": 92.0, "high": null}}')
    # custom table: 91 is anomalous; default table: 91 is fine
    assert is_anomalous("spo2", 91.0, custom) is True
    assert is_anomalous("spo2", 91.0, default) is False
    assert is_anomalous("spo2", 91.0) is False  # falls back to default


def test_parse_thresholds_invalid_falls_back():
    fallback = parse_thresholds("not json at all")
    assert fallback["spo2"]["low"] == 90.0
