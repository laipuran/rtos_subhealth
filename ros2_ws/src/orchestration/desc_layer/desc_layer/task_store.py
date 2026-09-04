from __future__ import annotations

import json
import os
import sqlite3
import threading
import time
from typing import Optional

from ros_interfaces.action import ExecTask

DB_NAME = "tasks.db"


class TaskRecord:
    def __init__(
        self,
        goal_id: str,
        goal: ExecTask.Goal,
        state: str = "accepted",
        progress: float = 0.0,
        current_tag: int = -1,
        next_tag: int = -1,
        error_code: str = "",
        message: str = "",
        result: Optional[ExecTask.Result] = None,
        route: Optional[list] = None,
        finished_stages: int = 0,
        created_at: Optional[float] = None,
        updated_at: Optional[float] = None,
    ) -> None:
        self.goal_id = goal_id
        self.goal = goal
        self.state = state
        self.progress = progress
        self.current_tag = current_tag
        self.next_tag = next_tag
        self.error_code = error_code
        self.message = message
        self.result = result
        self.route = list(route) if route else []
        self.finished_stages = finished_stages
        self.created_at = created_at or time.time()
        self.updated_at = updated_at or time.time()

    def to_dict(self) -> dict:
        return {
            "goal_id": self.goal_id,
            "type": self.goal.type,
            "priority": self.goal.priority,
            "route_id": self.goal.route_id,
            "target_tags": list(self.goal.target_tags),
            "state": self.state,
            "progress": self.progress,
            "current_tag": self.current_tag,
            "next_tag": self.next_tag,
            "error_code": self.error_code,
            "message": self.message,
            "final_state": self.result.final_state if self.result else None,
            "route": list(self.route or []),
            "finished_stages": self.finished_stages,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }

    def _to_row(self) -> dict:
        return {
            "goal_id": self.goal_id,
            "goal_json": json.dumps({
                "type": self.goal.type,
                "priority": self.goal.priority,
                "route_id": self.goal.route_id,
                "target_tags": list(self.goal.target_tags),
                "constraints": {
                    "max_speed_mps": self.goal.constraints.max_speed_mps,
                    "min_clearance_m": self.goal.constraints.min_clearance_m,
                    "avoid_tags": list(self.goal.constraints.avoid_tags),
                },
                "deadline_ms": self.goal.deadline_ms,
            }),
            "state": self.state,
            "progress": self.progress,
            "current_tag": self.current_tag,
            "next_tag": self.next_tag,
            "error_code": self.error_code,
            "message": self.message,
            "result_json": json.dumps({
                "final_state": self.result.final_state if self.result else "",
                "error_code": self.result.error_code if self.result else "",
                "message": self.result.message if self.result else "",
            }) if self.result else "",
            "route_json": json.dumps(list(self.route or [])),
            "finished_stages": self.finished_stages,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }


class TaskStore:
    def __init__(self, db_dir: str = ""):
        self._lock = threading.Lock()
        db_path = os.path.join(db_dir, DB_NAME) if db_dir else DB_NAME
        self._conn = sqlite3.connect(db_path, check_same_thread=False)
        self._conn.row_factory = sqlite3.Row
        self._conn.execute("""
            CREATE TABLE IF NOT EXISTS tasks (
                goal_id TEXT PRIMARY KEY,
                goal_json TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'accepted',
                progress REAL DEFAULT 0.0,
                current_tag INTEGER DEFAULT -1,
                next_tag INTEGER DEFAULT -1,
                error_code TEXT DEFAULT '',
                message TEXT DEFAULT '',
                result_json TEXT DEFAULT '',
                route_json TEXT DEFAULT '[]',
                finished_stages INTEGER DEFAULT 0,
                created_at REAL NOT NULL,
                updated_at REAL NOT NULL
            )
        """)
        self._ensure_column("route_json", "TEXT DEFAULT '[]'")
        self._ensure_column("finished_stages", "INTEGER DEFAULT 0")
        self._conn.commit()

    def _ensure_column(self, name: str, decl: str) -> None:
        """Add a column if the (older) DB table already exists without it."""
        cols = {r["name"] for r in self._conn.execute("PRAGMA table_info(tasks)")}
        if name not in cols:
            self._conn.execute(f"ALTER TABLE tasks ADD COLUMN {name} {decl}")

    def add(self, record: TaskRecord) -> None:
        with self._lock:
            row = record._to_row()
            self._conn.execute("""
                INSERT OR REPLACE INTO tasks
                (goal_id, goal_json, state, progress, current_tag, next_tag,
                 error_code, message, result_json, route_json, finished_stages,
                 created_at, updated_at)
                VALUES (:goal_id, :goal_json, :state, :progress, :current_tag,
                        :next_tag, :error_code, :message, :result_json,
                        :route_json, :finished_stages, :created_at, :updated_at)
            """, row)
            self._conn.commit()

    def get(self, goal_id: str) -> Optional[TaskRecord]:
        with self._lock:
            row = self._conn.execute(
                "SELECT * FROM tasks WHERE goal_id = ?", (goal_id,)
            ).fetchone()
            if row is None:
                return None
            return self._row_to_record(row)

    def update(
        self,
        goal_id: str,
        state: Optional[str] = None,
        progress: Optional[float] = None,
        current_tag: Optional[int] = None,
        next_tag: Optional[int] = None,
        error_code: Optional[str] = None,
        message: Optional[str] = None,
        result: Optional[ExecTask.Result] = None,
        route: Optional[list] = None,
        finished_stages: Optional[int] = None,
    ) -> None:
        with self._lock:
            fields = []
            values = []
            if state is not None:
                fields.append("state = ?")
                values.append(state)
            if progress is not None:
                fields.append("progress = ?")
                values.append(progress)
            if current_tag is not None:
                fields.append("current_tag = ?")
                values.append(current_tag)
            if next_tag is not None:
                fields.append("next_tag = ?")
                values.append(next_tag)
            if error_code is not None:
                fields.append("error_code = ?")
                values.append(error_code)
            if message is not None:
                fields.append("message = ?")
                values.append(message)
            if result is not None:
                fields.append("result_json = ?")
                values.append(json.dumps({
                    "final_state": result.final_state,
                    "error_code": result.error_code,
                    "message": result.message,
                }))
            if route is not None:
                fields.append("route_json = ?")
                values.append(json.dumps(list(route)))
            if finished_stages is not None:
                fields.append("finished_stages = ?")
                values.append(int(finished_stages))
            fields.append("updated_at = ?")
            values.append(time.time())
            values.append(goal_id)
            self._conn.execute(
                f"UPDATE tasks SET {', '.join(fields)} WHERE goal_id = ?",
                values,
            )
            self._conn.commit()

    def list_all(self) -> list[TaskRecord]:
        with self._lock:
            rows = self._conn.execute(
                "SELECT * FROM tasks ORDER BY created_at DESC"
            ).fetchall()
            return [self._row_to_record(r) for r in rows]

    def list_active(self) -> list[TaskRecord]:
        with self._lock:
            rows = self._conn.execute(
                "SELECT * FROM tasks WHERE state IN ('accepted', 'running') ORDER BY created_at DESC"
            ).fetchall()
            return [self._row_to_record(r) for r in rows]

    def exists(self, goal_id: str) -> bool:
        with self._lock:
            row = self._conn.execute(
                "SELECT 1 FROM tasks WHERE goal_id = ?", (goal_id,)
            ).fetchone()
            return row is not None

    def close(self) -> None:
        self._conn.close()

    @staticmethod
    def _row_to_record(row: sqlite3.Row) -> TaskRecord:
        gd = json.loads(row["goal_json"])
        goal = ExecTask.Goal()
        goal.type = gd.get("type", "")
        goal.priority = gd.get("priority", 0)
        goal.route_id = gd.get("route_id", "")
        goal.target_tags = gd.get("target_tags", [])
        c = gd.get("constraints", {})
        goal.constraints.max_speed_mps = c.get("max_speed_mps", 0.0)
        goal.constraints.min_clearance_m = c.get("min_clearance_m", 0.0)
        goal.constraints.avoid_tags = c.get("avoid_tags", [])
        goal.deadline_ms = gd.get("deadline_ms", 0)

        result = None
        rj = row["result_json"]
        if rj:
            rd = json.loads(rj)
            result = ExecTask.Result()
            result.final_state = rd.get("final_state", "")
            result.error_code = rd.get("error_code", "")
            result.message = rd.get("message", "")

        return TaskRecord(
            goal_id=row["goal_id"],
            goal=goal,
            state=row["state"],
            progress=row["progress"],
            current_tag=row["current_tag"],
            next_tag=row["next_tag"],
            error_code=row["error_code"],
            message=row["message"],
            result=result,
            route=json.loads(row["route_json"] or "[]"),
            finished_stages=row["finished_stages"],
            created_at=row["created_at"],
            updated_at=row["updated_at"],
        )
