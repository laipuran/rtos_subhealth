"""Pure deterministic behavior for the mock execution layer."""

from dataclasses import dataclass
from typing import Sequence

FALLBACK_GOAL_TAG = 42
FALLBACK_PATROL_ROUTE = (10, 20, 30)


class UnsupportedTask(ValueError):
    """Raised when a task type is not supported by the mock."""


@dataclass(frozen=True)
class StepFeedback:
    progress: float
    current_tag: int
    next_tag: int
    finished_stages: int


def route_for(task_type: str, target_tags: Sequence[int]) -> list[int]:
    if task_type == 'hold':
        return []
    if task_type == 'go_to_tag':
        return [target_tags[0] if target_tags else FALLBACK_GOAL_TAG]
    if task_type == 'patrol_route':
        return list(target_tags or FALLBACK_PATROL_ROUTE)
    raise UnsupportedTask(task_type)


def step_feedback(route: Sequence[int], index: int) -> StepFeedback:
    if index < 0 or index >= len(route):
        raise IndexError(index)
    current_tag = route[index]
    return StepFeedback(
        progress=(index + 1) / len(route),
        current_tag=current_tag,
        next_tag=route[index + 1] if index + 1 < len(route) else -1,
        finished_stages=index + 1,
    )
