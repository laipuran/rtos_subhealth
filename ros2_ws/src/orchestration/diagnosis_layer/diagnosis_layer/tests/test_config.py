from __future__ import annotations

import os
import importlib
import tempfile

from diagnosis_layer.config import build_config, load_config_file


def _setenv(env: dict):
    saved = {}
    for k, v in env.items():
        saved[k] = os.environ.get(k)
        os.environ[k] = v
    return saved


def _restoreenv(saved):
    for k, v in saved.items():
        if v is None:
            os.environ.pop(k, None)
        else:
            os.environ[k] = v


def test_defaults_no_input():
    cfg = build_config({}, "")
    assert cfg["llm_base_url"] == ""
    assert cfg["llm_model"] == "gpt-4o-mini"
    assert cfg["embedding_model"] == "text-embedding-3-small"


def test_env_override():
    saved = _setenv({"LLM_API_KEY": "sk-test", "LLM_BASE_URL": "http://x/v1"})
    try:
        cfg = build_config({}, "")
        assert cfg["llm_api_key"] == "sk-test"
        assert cfg["llm_base_url"] == "http://x/v1"
    finally:
        _restoreenv(saved)


def test_openai_shorthand():
    saved = _setenv({"OPENAI_API_KEY": "sk-test"})
    try:
        cfg = build_config({}, "")
        assert cfg["llm_api_key"] == "sk-test"
        assert cfg["llm_base_url"] == "https://api.openai.com"
    finally:
        _restoreenv(saved)


def test_ros_param_wins_over_env():
    saved = _setenv({"LLM_API_KEY": "env-key"})
    try:
        cfg = build_config({"llm_api_key": "param-key"}, "")
        assert cfg["llm_api_key"] == "param-key"
    finally:
        _restoreenv(saved)


def test_config_file_loaded():
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False, encoding="utf-8") as f:
        f.write('{"llm_base_url": "http://file/v1", "llm_model": "llama3"}')
        path = f.name
    try:
        cfg = build_config({}, path)
        assert cfg["llm_base_url"] == "http://file/v1"
        assert cfg["llm_model"] == "llama3"
    finally:
        os.unlink(path)
