from __future__ import annotations

import logging
import os
import re
from typing import List, Optional

from .corpus import Chunk, load_corpus
from .embeddings import EmbeddingClient

logger = logging.getLogger(__name__)

_DATA_TYPE_KEYWORDS = {
    "spo2": ["spo2", "血氧", "oxygen", "oxygenation"],
    "heart_rate": ["heart rate", "心率", "pulse", "bpm"],
    "systolic_mmhg": ["systolic", "收缩压", "blood pressure"],
    "diastolic_mmhg": ["diastolic", "舒张压", "blood pressure"],
    "body_temp_c": ["body temperature", "体温", "fever", "temperature"],
    "respiratory_rate": ["respiratory", "呼吸", "respiration", "rr"],
}

_STOPWORDS = set(
    "the a an of to and or in for is are be with on at by this that you your".split()
)


class Retriever:
    """Retrieve top-k medical references for a physiological snapshot.

    Two backends are supported:
      * ``embeddings``: semantic search via an OpenAI-compatible embed API.
      * ``keyword``:    zero-dependency keyword/rule fallback (default when no
                        embedding client is supplied or the index is unavailable).
    """

    def __init__(
        self,
        corpus_dir: Optional[str] = None,
        top_k: int = 3,
        embedding: Optional[EmbeddingClient] = None,
        min_keyword_score: float = 0.0,
    ) -> None:
        self._corpus_dir = corpus_dir or _default_corpus_dir()
        self._top_k = top_k
        self._embedding = embedding
        self._min_keyword_score = min_keyword_score
        self._chunks: List[Chunk] = []
        self._embeddings: Optional[List[List[float]]] = None
        self.reload()

    @property
    def mode(self) -> str:
        return "embeddings" if self._embeddings is not None else "keyword"

    @property
    def chunk_count(self) -> int:
        return len(self._chunks)

    def reload(self) -> None:
        """Reload the corpus and (re)build the embedding index if possible."""
        self._chunks = load_corpus(self._corpus_dir)
        self._embeddings = None
        if self._embedding is not None and self._chunks:
            try:
                self._embeddings = self._embedding.embed([c.text for c in self._chunks])
                logger.info("Built embedding index with %d chunks", len(self._chunks))
            except Exception as exc:  # noqa: BLE001
                logger.warning("Embedding index unavailable, keyword fallback: %s", exc)
                self._embeddings = None

    def retrieve(self, snapshot: dict) -> List[str]:
        """Return up to ``top_k`` relevant reference texts for ``snapshot``."""
        if not self._chunks:
            return []
        query = self.build_query(snapshot)
        if self._embeddings is not None:
            return self._retrieve_embeddings(query)
        return self._retrieve_keyword(query, snapshot)

    @staticmethod
    def format_context(chunks: List[str], max_chars: int = 4000) -> str:
        """Join retrieved chunks into a single prompt context block."""
        parts: List[str] = []
        total = 0
        for chunk in chunks:
            if total + len(chunk) > max_chars:
                break
            parts.append(chunk)
            total += len(chunk)
        return "\n\n".join(parts)

    def build_query(self, snapshot: dict) -> str:
        parts: List[str] = []
        for src in snapshot.get("sources", []):
            data_type = src.get("data_type", "")
            parts.extend(_DATA_TYPE_KEYWORDS.get(data_type, [data_type]))
            if src.get("mean") is not None:
                parts.append(f"{data_type} {src['mean']}")
            if not src.get("valid", True):
                parts.append("sensor abnormal")
        if snapshot.get("trigger_type"):
            parts.append(str(snapshot["trigger_type"]))
        return " ".join(parts)

    def _retrieve_embeddings(self, query: str) -> List[str]:
        import numpy as np

        q = np.asarray(self._embedding.embed([query])[0], dtype=float)
        qn = np.linalg.norm(q)
        if qn > 0:
            q = q / qn
        sims: List[float] = []
        for vec in self._embeddings:  # type: ignore[union-attr]
            v = np.asarray(vec, dtype=float)
            vn = np.linalg.norm(v)
            if vn > 0:
                v = v / vn
            sims.append(float(np.dot(q, v)))
        ranked = sorted(range(len(sims)), key=lambda i: sims[i], reverse=True)[: self._top_k]
        return [self._chunks[i].text for i in ranked]

    def _retrieve_keyword(self, query: str, snapshot: dict) -> List[str]:
        tokens = _tokenize(query)
        dtype_terms: set = set()
        for src in snapshot.get("sources", []):
            dtype_terms.update(_DATA_TYPE_KEYWORDS.get(src.get("data_type", ""), []))
        scored = []
        for chunk in self._chunks:
            text_l = chunk.text.lower()
            score = sum(1 for t in tokens if t in text_l)
            score += sum(2 for term in dtype_terms if term.lower() in text_l)
            if score > self._min_keyword_score:
                scored.append((score, chunk.text))
        scored.sort(key=lambda x: x[0], reverse=True)
        return [text for _score, text in scored[: self._top_k]]


def _tokenize(text: str) -> List[str]:
    tokens = re.findall(r"[a-z0-9_]+", text.lower())
    return [t for t in tokens if t not in _STOPWORDS and len(t) > 1]


def _default_corpus_dir() -> str:
    env = os.environ.get("MEDICAL_CORPUS_DIR")
    if env:
        return env
    here = os.path.dirname(os.path.abspath(__file__))
    repo_root = _find_repo_root(here)
    return os.path.normpath(os.path.join(repo_root, "docs", "medical"))


def _find_repo_root(start: str) -> str:
    cur = start
    for _ in range(8):
        if os.path.isdir(os.path.join(cur, "docs")):
            return cur
        parent = os.path.dirname(cur)
        if parent == cur:
            break
        cur = parent
    return start
