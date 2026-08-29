from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional

# (low, high) outside which the metric is considered abnormal. None means no bound.
ANOMALY_THRESHOLDS: Dict[str, Dict[str, Optional[float]]] = {
    "spo2": {"low": 90.0, "high": None},
    "heart_rate": {"low": 60.0, "high": 100.0},
    "systolic_mmhg": {"low": None, "high": 140.0},
    "diastolic_mmhg": {"low": None, "high": 90.0},
    "body_temp_c": {"low": 35.0, "high": 37.3},
    "respiratory_rate": {"low": 12.0, "high": 20.0},
}


@dataclass
class Sample:
    t: float
    value: float
    valid: bool = True


@dataclass
class Window:
    data_src: str
    data_type: str
    window_seconds: float = 60.0
    _samples: List[Sample] = field(default_factory=list)

    def add(self, sample: Sample) -> None:
        self._samples.append(sample)

    def prune(self, now: float) -> None:
        cutoff = now - self.window_seconds
        self._samples = [s for s in self._samples if s.t >= cutoff]

    @property
    def samples(self) -> List[Sample]:
        return self._samples

    def stats(self) -> Dict:
        valid = [s for s in self._samples if s.valid]
        values = [s.value for s in valid]
        if not values:
            return {
                "count": len(self._samples),
                "valid_count": 0,
                "mean": None,
                "min": None,
                "max": None,
                "latest": None,
                "trend": "unknown",
                "valid": False,
            }
        mean = sum(values) / len(values)
        latest = values[-1]
        trend = _trend(valid)
        return {
            "count": len(self._samples),
            "valid_count": len(valid),
            "mean": round(mean, 2),
            "min": round(min(values), 2),
            "max": round(max(values), 2),
            "latest": round(latest, 2),
            "trend": trend,
            "valid": True,
        }


def _trend(valid: List[Sample]) -> str:
    n = len(valid)
    if n < 4:
        return "unknown"
    half = n // 2
    older = sum(s.value for s in valid[:half]) / half
    recent = sum(s.value for s in valid[half:]) / (n - half)
    delta = recent - older
    if abs(delta) < 0.5:
        return "stable"
    return "increasing" if delta > 0 else "decreasing"


def is_anomalous(data_type: str, value: float,
                 thresholds: Optional[Dict[str, Dict[str, Optional[float]]]] = None) -> bool:
    """Rule-based single-sample anomaly check per RFC-009 §6.

    ``thresholds`` overrides the per-data_type (low, high) bounds; falls back to
    the default ``ANOMALY_THRESHOLDS`` when omitted (so the node can configure
    them via ROS parameters, RFC-009 §9.3).
    """
    rule = (thresholds or ANOMALY_THRESHOLDS).get(data_type)
    if rule is None:
        return False
    low = rule.get("low")
    high = rule.get("high")
    if low is not None and value < low:
        return True
    if high is not None and value > high:
        return True
    return False


def parse_thresholds(json_str: str) -> Dict[str, Dict[str, Optional[float]]]:
    """Parse a JSON threshold table (RFC-009 §9.3 configurable thresholds).

    Expected shape: {"spo2": {"low": 90.0, "high": null}, ...}.
    Returns the default table when ``json_str`` is empty/invalid.
    """
    if not json_str:
        return dict(ANOMALY_THRESHOLDS)
    try:
        import json as _json
        parsed = _json.loads(json_str)
        if not isinstance(parsed, dict):
            return dict(ANOMALY_THRESHOLDS)
        norm: Dict[str, Dict[str, Optional[float]]] = {}
        for k, v in parsed.items():
            if not isinstance(v, dict):
                continue
            norm[k] = {
                "low": v.get("low"),
                "high": v.get("high"),
            }
        return norm or dict(ANOMALY_THRESHOLDS)
    except Exception:
        return dict(ANOMALY_THRESHOLDS)


def build_snapshot(windows: Dict[str, Window], trigger_type: str) -> Dict:
    """Assemble a multi-source structured snapshot for RAG + LLM."""
    sources = []
    for src, win in windows.items():
        st = win.stats()
        sources.append({
            "data_src": src,
            "data_type": win.data_type,
            "mean": st["mean"],
            "min": st["min"],
            "max": st["max"],
            "latest": st["latest"],
            "trend": st["trend"],
            "valid": st["valid"],
        })
    return {"trigger_type": trigger_type, "sources": sources}
