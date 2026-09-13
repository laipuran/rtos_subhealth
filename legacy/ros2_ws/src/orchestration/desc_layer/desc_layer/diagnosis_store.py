from __future__ import annotations

import json
import os
import sqlite3
import threading
import time
from typing import List, Optional

DB_NAME = "diagnoses.db"


class DiagnosisRecord:
    def __init__(
        self,
        diagnosis_id: str,
        source_ids: List[str],
        trigger_type: str,
        severity: str = "normal",
        summary: str = "",
        possible_causes: Optional[List[str]] = None,
        recommendations: Optional[List[str]] = None,
        confidence: float = 0.0,
        disclaimer: str = "",
        raw_prompt: str = "",
        error_code: str = "",
        error_message: str = "",
        metrics: Optional[List[dict]] = None,
        created_at: Optional[float] = None,
    ) -> None:
        self.diagnosis_id = diagnosis_id
        self.source_ids = source_ids
        self.trigger_type = trigger_type
        self.severity = severity
        self.summary = summary
        self.possible_causes = possible_causes or []
        self.recommendations = recommendations or []
        self.confidence = confidence
        self.disclaimer = disclaimer
        self.raw_prompt = raw_prompt
        self.error_code = error_code
        self.error_message = error_message
        self.metrics = metrics or []
        self.created_at = created_at or time.time()

    def to_dict(self) -> dict:
        return {
            "diagnosis_id": self.diagnosis_id,
            "source_ids": list(self.source_ids),
            "trigger_type": self.trigger_type,
            "severity": self.severity,
            "summary": self.summary,
            "possible_causes": list(self.possible_causes),
            "recommendations": list(self.recommendations),
            "confidence": self.confidence,
            "disclaimer": self.disclaimer,
            "raw_prompt": self.raw_prompt,
            "error_code": self.error_code,
            "error_message": self.error_message,
            "metrics": list(self.metrics or []),
            "created_at": self.created_at,
        }


class DiagnosisStore:
    def __init__(self, db_dir: str = ""):
        self._lock = threading.Lock()
        db_path = os.path.join(db_dir, DB_NAME) if db_dir else DB_NAME
        self._conn = sqlite3.connect(db_path, check_same_thread=False)
        self._conn.row_factory = sqlite3.Row
        self._conn.execute("""
            CREATE TABLE IF NOT EXISTS diagnoses (
                diagnosis_id TEXT PRIMARY KEY,
                source_ids TEXT NOT NULL,
                trigger_type TEXT NOT NULL,
                severity TEXT NOT NULL DEFAULT 'normal',
                summary TEXT DEFAULT '',
                possible_causes TEXT DEFAULT '[]',
                recommendations TEXT DEFAULT '[]',
                confidence REAL DEFAULT 0.0,
                disclaimer TEXT DEFAULT '',
                raw_prompt TEXT DEFAULT '',
                error_code TEXT DEFAULT '',
                error_message TEXT DEFAULT '',
                metrics_json TEXT DEFAULT '[]',
                created_at REAL NOT NULL
            )
        """)
        self._ensure_column("metrics_json", "TEXT DEFAULT '[]'")
        self._conn.commit()

    def _ensure_column(self, name: str, decl: str) -> None:
        """Add a column if the (older) DB table already exists without it."""
        cols = {r["name"] for r in self._conn.execute("PRAGMA table_info(diagnoses)")}
        if name not in cols:
            self._conn.execute(f"ALTER TABLE diagnoses ADD COLUMN {name} {decl}")

    def add(self, rec: DiagnosisRecord) -> None:
        with self._lock:
            self._conn.execute("""
                INSERT OR REPLACE INTO diagnoses
                (diagnosis_id, source_ids, trigger_type, severity, summary,
                 possible_causes, recommendations, confidence, disclaimer,
                 raw_prompt, error_code, error_message, metrics_json, created_at)
                VALUES (:diagnosis_id, :source_ids, :trigger_type, :severity, :summary,
                        :possible_causes, :recommendations, :confidence, :disclaimer,
                        :raw_prompt, :error_code, :error_message, :metrics_json, :created_at)
            """, {
                "diagnosis_id": rec.diagnosis_id,
                "source_ids": json.dumps(list(rec.source_ids)),
                "trigger_type": rec.trigger_type,
                "severity": rec.severity,
                "summary": rec.summary,
                "possible_causes": json.dumps(list(rec.possible_causes)),
                "recommendations": json.dumps(list(rec.recommendations)),
                "confidence": rec.confidence,
                "disclaimer": rec.disclaimer,
                "raw_prompt": rec.raw_prompt,
                "error_code": rec.error_code,
                "error_message": rec.error_message,
                "metrics_json": json.dumps(list(rec.metrics or [])),
                "created_at": rec.created_at,
            })
            self._conn.commit()

    def get(self, diagnosis_id: str) -> Optional[DiagnosisRecord]:
        with self._lock:
            row = self._conn.execute(
                "SELECT * FROM diagnoses WHERE diagnosis_id = ?", (diagnosis_id,)
            ).fetchone()
            return self._row_to_record(row) if row else None

    def list_all(self, offset: int = 0, limit: int = 50) -> List[DiagnosisRecord]:
        with self._lock:
            rows = self._conn.execute(
                "SELECT * FROM diagnoses ORDER BY created_at DESC LIMIT ? OFFSET ?",
                (limit, offset),
            ).fetchall()
            return [self._row_to_record(r) for r in rows]

    def count(self) -> int:
        with self._lock:
            return self._conn.execute("SELECT COUNT(*) FROM diagnoses").fetchone()[0]

    def purge_older_than(self, max_age_s: float) -> int:
        """Delete diagnoses older than ``max_age_s`` seconds. Returns deleted count.

        RFC-009 §9.4: retention / cleanup policy. ``max_age_s <= 0`` is a no-op
        (keep forever).
        """
        if max_age_s <= 0:
            return 0
        cutoff = time.time() - max_age_s
        with self._lock:
            cur = self._conn.execute(
                "DELETE FROM diagnoses WHERE created_at < ?", (cutoff,))
            self._conn.commit()
            return cur.rowcount

    def close(self) -> None:
        self._conn.close()

    @staticmethod
    def _row_to_record(row: sqlite3.Row) -> DiagnosisRecord:
        return DiagnosisRecord(
            diagnosis_id=row["diagnosis_id"],
            source_ids=json.loads(row["source_ids"]),
            trigger_type=row["trigger_type"],
            severity=row["severity"],
            summary=row["summary"],
            possible_causes=json.loads(row["possible_causes"]),
            recommendations=json.loads(row["recommendations"]),
            confidence=row["confidence"],
            disclaimer=row["disclaimer"],
            raw_prompt=row["raw_prompt"],
            error_code=row["error_code"],
            error_message=row["error_message"],
            metrics=json.loads(row["metrics_json"] or "[]"),
            created_at=row["created_at"],
        )
