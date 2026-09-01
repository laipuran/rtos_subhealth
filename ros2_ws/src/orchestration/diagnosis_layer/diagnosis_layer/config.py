from __future__ import annotations

import json
import os
from typing import Dict, Optional


# 默认值：全部留空表示"未接入"（节点走 LLM_DISABLED / 关键词回退）
DEFAULTS: Dict[str, object] = {
    "llm_base_url": "",
    "llm_api_key": "",
    "llm_model": "gpt-4o-mini",
    "embedding_base_url": "",
    "embedding_api_key": "",
    "embedding_model": "text-embedding-3-small",
    "medical_corpus_dir": "",
    "confidence_min": 0.8,
    "rag_top_k": 3,
}

# 环境变量名 -> 配置键 的映射（env 优先级低于 ROS 参数 / 配置文件）
_ENV_MAP = {
    "LLM_BASE_URL": "llm_base_url",
    "LLM_API_KEY": "llm_api_key",
    "LLM_MODEL": "llm_model",
    "EMBEDDING_BASE_URL": "embedding_base_url",
    "EMBEDDING_API_KEY": "embedding_api_key",
    "EMBEDDING_MODEL": "embedding_model",
    "OPENAI_BASE_URL": "llm_base_url",      # OpenAI 兼容速记
    "OPENAI_API_KEY": "llm_api_key",
    "OPENAI_MODEL": "llm_model",
    "MEDICAL_CORPUS_DIR": "medical_corpus_dir",
}


def load_config_file(path: str) -> Dict[str, object]:
    """Load a JSON config file (RFC-009 AI 接入配置)。缺失/非法返回空 dict。"""
    if not path or not os.path.isfile(path):
        return {}
    try:
        with open(path, "r", encoding="utf-8") as fh:
            data = json.load(fh)
        return {k: v for k, v in data.items() if k in DEFAULTS}
    except Exception:
        return {}


def resolve(settings: Dict[str, object]) -> Dict[str, object]:
    """Apply OpenAI shorthand: 只给 key 时自动补 base_url（不带 /v1）。"""
    out = dict(settings)
    if out.get("llm_api_key") and not out.get("llm_base_url"):
        out["llm_base_url"] = "https://api.openai.com"
    if out.get("embedding_api_key") and not out.get("embedding_base_url"):
        out["embedding_base_url"] = "https://api.openai.com"
    return out


def build_config(ros_params: Dict[str, object],
                 config_file: str = "") -> Dict[str, object]:
    """Merge: DEFAULTS < JSON file < ROS params; env vars fill gaps only.

    环境变量可覆盖 DEFAULTS，但不覆盖显式设置的 ROS 参数。
    """
    cfg: Dict[str, object] = dict(DEFAULTS)
    cfg.update(load_config_file(config_file))      # 配置文件
    ros_set: set = set()                           # 记录哪些 key 被 ROS 参数显式设置
    for k, v in ros_params.items():               # ROS 参数优先于文件
        if v not in (None, ""):
            cfg[k] = v
            ros_set.add(k)
    for env_key, cfg_key in _ENV_MAP.items():      # 环境变量兜底（不覆盖 ROS 参数）
        if cfg_key in ros_set:
            continue
        val = os.environ.get(env_key)
        if val:
            cfg[cfg_key] = val
    return resolve(cfg)
