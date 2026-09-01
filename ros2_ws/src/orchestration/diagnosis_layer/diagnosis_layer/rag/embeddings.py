from __future__ import annotations

import requests
from typing import List


class EmbeddingClient:
    """Thin OpenAI-compatible ``/v1/embeddings`` client.

    Used by the RAG retriever when an embedding backend is configured. Any
    failure propagates so the retriever can fall back to keyword search
    (RFC-009 risk 2).
    """

    def __init__(self, base_url: str, api_key: str, model: str, timeout: float = 10.0) -> None:
        self._base_url = base_url.rstrip("/")
        if self._base_url.endswith("/v1"):
            self._base_url = self._base_url[:-3]
        self._api_key = api_key
        self._model = model
        self._timeout = timeout

    def embed(self, texts: List[str]) -> List[List[float]]:
        if not texts:
            return []
        resp = requests.post(
            f"{self._base_url}/v1/embeddings",
            headers={"Authorization": f"Bearer {self._api_key}"},
            json={"model": self._model, "input": texts},
            timeout=self._timeout,
        )
        resp.raise_for_status()
        data = resp.json().get("data", [])
        return [item["embedding"] for item in data]
