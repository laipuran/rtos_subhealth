from __future__ import annotations

import enum
import time
from typing import Optional

from ros_interfaces.msg import Segment


class LostTagLevel(enum.IntEnum):
    NONE = -1
    LEVEL_1 = 0
    LEVEL_2 = 1
    LEVEL_3 = 2


class RecoveryAction(enum.Enum):
    CONTINUE = "continue"
    SEARCH = "search"
    FAIL_SEGMENT = "fail_segment"


class SegmentFailureAction(enum.Enum):
    RETRY = "retry"
    REROUTE = "reroute"
    FAIL_TASK = "fail_task"


class RecoveryHandler:
    LOST_LEVEL_1_TIMEOUT = 2.0
    LOST_LEVEL_2_TIMEOUT = 5.0
    MAX_RETRIES = 3
    RETRY_WAIT_SEC = 3.0

    def __init__(self) -> None:
        self._lost_start_time: Optional[float] = None
        self._current_level = LostTagLevel.NONE
        self._retry_count: dict[str, int] = {}

    def handle_lost_tag(self) -> RecoveryAction:
        if self._lost_start_time is None:
            self._lost_start_time = time.time()
            self._current_level = LostTagLevel.LEVEL_1

        lost_duration = time.time() - self._lost_start_time

        if lost_duration <= self.LOST_LEVEL_1_TIMEOUT:
            self._current_level = LostTagLevel.LEVEL_1
            return RecoveryAction.CONTINUE
        elif lost_duration <= self.LOST_LEVEL_2_TIMEOUT:
            self._current_level = LostTagLevel.LEVEL_2
            return RecoveryAction.SEARCH
        else:
            self._current_level = LostTagLevel.LEVEL_3
            return RecoveryAction.FAIL_SEGMENT

    def on_tag_found(self) -> None:
        self._lost_start_time = None
        self._current_level = LostTagLevel.NONE

    def get_current_level(self) -> LostTagLevel:
        return self._current_level

    def handle_segment_failure(
        self, segment: Segment,
    ) -> SegmentFailureAction:
        edge_key = segment.edge_id or f"{segment.from_tag}->{segment.to_tag}"
        self._retry_count[edge_key] = self._retry_count.get(edge_key, 0) + 1
        retry_num = self._retry_count[edge_key]

        if retry_num < self.MAX_RETRIES:
            return SegmentFailureAction.RETRY
        return SegmentFailureAction.REROUTE

    def get_retry_wait(self) -> float:
        return self.RETRY_WAIT_SEC

    def reset_retries(self, segment: Segment) -> None:
        edge_key = segment.edge_id or f"{segment.from_tag}->{segment.to_tag}"
        self._retry_count.pop(edge_key, None)

    def clear(self) -> None:
        self._lost_start_time = None
        self._current_level = LostTagLevel.NONE
        self._retry_count.clear()
