from __future__ import annotations

import threading
import time
from dataclasses import dataclass, field
from typing import Optional

from ros_interfaces.action import ExecTask


@dataclass
class TaskRecord:
    goal_id: str
    goal: ExecTask.Goal
    state: str = "accepted"  # accepted / running / succeeded / failed / canceled
    progress: float = 0.0
    current_tag: int = -1
    next_tag: int = -1
    error_code: str = ""
    message: str = ""
    result: Optional[ExecTask.Result] = None
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)

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
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }


class TaskStore:
    def __init__(self):
        self._lock = threading.Lock()
        self._tasks: dict[str, TaskRecord] = {}

    def add(self, record: TaskRecord) -> None:
        with self._lock:
            self._tasks[record.goal_id] = record

    def get(self, goal_id: str) -> Optional[TaskRecord]:
        with self._lock:
            return self._tasks.get(goal_id)

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
    ) -> None:
        with self._lock:
            rec = self._tasks.get(goal_id)
            if rec is None:
                return
            if state is not None:
                rec.state = state
            if progress is not None:
                rec.progress = progress
            if current_tag is not None:
                rec.current_tag = current_tag
            if next_tag is not None:
                rec.next_tag = next_tag
            if error_code is not None:
                rec.error_code = error_code
            if message is not None:
                rec.message = message
            if result is not None:
                rec.result = result
            rec.updated_at = time.time()

    def list_all(self) -> list[TaskRecord]:
        with self._lock:
            return list(self._tasks.values())

    def exists(self, goal_id: str) -> bool:
        with self._lock:
            return goal_id in self._tasks
