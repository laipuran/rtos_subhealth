from __future__ import annotations

from collections import defaultdict, deque
from typing import Optional

from apriltag_interfaces.msg import AprilTagDetection, AprilTagDetections


class DetectionFilter:
    _history: dict[int, deque[AprilTagDetection]]
    _window: int

    def __init__(self, window: int = 10) -> None:
        self._history = defaultdict(lambda: deque(maxlen=window))
        self._window = window

    def update(self, msg: AprilTagDetections) -> None:
        seen = set()
        for det in msg.detections:
            self._history[det.id].append(det)
            seen.add(det.id)
        for tag_id in list(self._history.keys()):
            if tag_id not in seen:
                self._history[tag_id].append(None)

    def is_stable(self, tag_id: int, min_frames: int = 6) -> bool:
        if tag_id not in self._history:
            return False
        recent = list(self._history[tag_id])
        valid = sum(1 for d in recent if d is not None)
        return valid >= min_frames

    def get_best(self, tag_id: int) -> Optional[AprilTagDetection]:
        if not self.is_stable(tag_id):
            return None
        candidates = [d for d in self._history[tag_id] if d is not None]
        if not candidates:
            return None
        return min(candidates, key=lambda d: d.distance)

    def get_most_frequent(self, n_frames: int = 10) -> Optional[int]:
        freq: dict[int, int] = {}
        for tag_id, buf in self._history.items():
            recent = list(buf)[-n_frames:]
            count = sum(1 for d in recent if d is not None)
            if count > 0:
                freq[tag_id] = count

        if not freq:
            return None
        return max(freq, key=freq.get)

    def get_recent_detections(self, tag_id: int) -> list[Optional[AprilTagDetection]]:
        return list(self._history.get(tag_id, []))

    def clear(self) -> None:
        self._history.clear()
