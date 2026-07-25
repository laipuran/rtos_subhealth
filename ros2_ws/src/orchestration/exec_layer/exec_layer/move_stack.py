from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional

from ros_interfaces.msg import Segment


@dataclass
class CompletedSegment:
    segment: Segment
    tag_id: int


class MoveStack:
    _completed: list[CompletedSegment]
    _in_progress: Optional[Segment]

    def __init__(self) -> None:
        self._completed = []
        self._in_progress = None

    def push(self, segment: Segment) -> None:
        if self._in_progress is not None:
            self._in_progress = None
        self._in_progress = segment

    def mark_completed(self, tag_id: int) -> None:
        if self._in_progress is not None:
            self._completed.append(CompletedSegment(
                segment=self._in_progress, tag_id=tag_id,
            ))
            self._in_progress = None

    def pop(self) -> Optional[CompletedSegment]:
        if not self._completed:
            return None
        return self._completed.pop()

    def get_rollback_path(self, blocked_tags: set[int]) -> list[Segment]:
        rollback: list[Segment] = []
        for cs in reversed(self._completed):
            if cs.tag_id in blocked_tags:
                continue
            rollback.append(Segment(
                from_tag=cs.segment.to_tag,
                to_tag=cs.segment.from_tag,
                edge_cost=cs.segment.edge_cost,
                edge_id=f"rollback_{cs.segment.edge_id}",
            ))
        return rollback

    def current_position_tag(self) -> Optional[int]:
        if self._completed:
            return self._completed[-1].tag_id
        return None

    def clear(self) -> None:
        self._completed.clear()
        self._in_progress = None

    def is_empty(self) -> bool:
        return len(self._completed) == 0 and self._in_progress is None

    def __len__(self) -> int:
        return len(self._completed) + (1 if self._in_progress is not None else 0)
