from __future__ import annotations

import json
import logging
import re
from typing import Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)

_REQUIRED = ["severity", "summary", "possible_causes", "recommendations", "confidence", "disclaimer"]
_SEVERITY_LEVELS = {"normal", "mild", "moderate", "severe", "critical"}


class LLMClient:
    """OpenAI-compatible chat completions client (RFC-009 §5.5)."""

    def __init__(self, base_url: str = "", api_key: str = "", model: str = "gpt-4o-mini",
                 timeout: float = 30.0) -> None:
        self._base_url = (base_url or "").rstrip("/")
        if self._base_url.endswith("/v1"):
            self._base_url = self._base_url[:-3]
        self._api_key = api_key
        self._model = model
        self._timeout = timeout

    @property
    def enabled(self) -> bool:
        return bool(self._base_url)

    def complete(self, system: str, user: str) -> str:
        import os
        import requests

        if not self.enabled:
            raise RuntimeError("LLM base_url not configured")
        url = f"{self._base_url}/v1/chat/completions"
        proxies = {}
        http_proxy = os.environ.get("HTTP_PROXY") or os.environ.get("http_proxy")
        https_proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("https_proxy")
        if http_proxy:
            proxies["http"] = http_proxy
        if https_proxy:
            proxies["https"] = https_proxy
        logger.info("LLM request: %s model=%s proxy=%s", url, self._model, proxies)
        print(f"[LLM] POST {url} model={self._model} proxy={proxies}", flush=True)
        resp = requests.post(
            url,
            headers={"Authorization": f"Bearer {self._api_key}"},
            json={
                "model": self._model,
                "temperature": 0.2,
                "response_format": {"type": "json_object"},
                "messages": [
                    {"role": "system", "content": system},
                    {"role": "user", "content": user},
                ],
            },
            timeout=self._timeout,
            proxies=proxies,
        )
        if not resp.ok:
            logger.warning("LLM %s %s -> %s: %s", resp.status_code,
                           url, resp.text[:500])
        resp.raise_for_status()
        return resp.json()["choices"][0]["message"]["content"]


def build_messages(snapshot: Dict, context: str) -> Tuple[str, str]:
    """Build (system, user) prompt injecting RAG context as constraints."""
    system = (
        "你是医学辅助分析助手。仅依据下面提供的【参考资料】给出健康建议级（advisory）"
        "分析，不得编造资料之外的结论，不做医疗诊断或报警。必须输出严格 JSON，字段为："
        "severity(枚举 normal|mild|moderate|severe|critical)、summary(中文简述)、"
        "possible_causes(字符串数组)、recommendations(字符串数组)、confidence(0-1 浮点)、"
        "disclaimer(免责声明)。"
    )
    sources = snapshot.get("sources", [])
    lines = [f"- {s.get('data_type')}: mean={s.get('mean')} min={s.get('min')} "
             f"max={s.get('max')} latest={s.get('latest')} trend={s.get('trend')} "
             f"valid={s.get('valid')}" for s in sources]
    user = (
        f"触发类型: {snapshot.get('trigger_type')}\n"
        f"体征快照:\n" + "\n".join(lines) + "\n\n"
        f"【参考资料】\n{context or '（无可用资料）'}\n\n"
        "请基于上述资料输出 JSON。"
    )
    return system, user


def _extract_json(text: str) -> Dict:
    text = text.strip()
    # strip ```json ... ``` fences
    m = re.search(r"```(?:json)?\s*(.*?)```", text, re.DOTALL)
    if m:
        text = m.group(1).strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        # try to salvage the first {...} block
        m2 = re.search(r"\{.*\}", text, re.DOTALL)
        if not m2:
            raise ValueError("no JSON object found in LLM output")
        return json.loads(m2.group(0))


def parse_diagnosis(content: str) -> Dict:
    """Parse + structurally validate LLM JSON output. Raises on invalid."""
    obj = _extract_json(content)
    if not isinstance(obj, dict):
        raise ValueError("LLM output is not a JSON object")
    missing = [k for k in _REQUIRED if k not in obj]
    if missing:
        raise ValueError(f"missing fields: {missing}")
    if obj.get("severity") not in _SEVERITY_LEVELS:
        raise ValueError(f"invalid severity: {obj.get('severity')}")
    try:
        obj["confidence"] = float(obj["confidence"])
    except (TypeError, ValueError):
        raise ValueError("confidence is not a number")
    if not 0.0 <= obj["confidence"] <= 1.0:
        raise ValueError(f"confidence {obj['confidence']} out of range [0, 1]")
    if not isinstance(obj.get("possible_causes"), list):
        obj["possible_causes"] = [str(obj.get("possible_causes", ""))]
    if not isinstance(obj.get("recommendations"), list):
        obj["recommendations"] = [str(obj.get("recommendations", ""))]
    return obj


def passes_confidence(obj: Dict, confidence_min: float) -> bool:
    return float(obj.get("confidence", 0.0)) >= confidence_min
