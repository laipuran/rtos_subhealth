"""Pure deterministic behavior for the mock execution layer."""

from dataclasses import dataclass
from enum import Enum, auto
import threading
from typing import Sequence
import weakref

FALLBACK_GOAL_TAG = 42
FALLBACK_PATROL_ROUTE = (10, 20, 30)


class UnsupportedTask(ValueError):
    """Raised when a task type is not supported by the mock."""


class _GoalTerminalState(Enum):
    ACTIVE = auto()
    CANCEL_ACCEPTED = auto()
    SUCCEEDED = auto()
    ABORTED = auto()
    CANCELED = auto()


class GoalTerminalCoordinator:
    """Atomically choose one terminal outcome for each live goal handle."""

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._states = {}

    def request_cancel(self, goal_handle) -> bool:
        with self._lock:
            state = self._state_for(goal_handle)
            if state is not _GoalTerminalState.ACTIVE:
                return False
            self._set_state(goal_handle, _GoalTerminalState.CANCEL_ACCEPTED)
            return True

    def is_cancel_pending(self, goal_handle) -> bool:
        with self._lock:
            return (
                self._state_for(goal_handle)
                is _GoalTerminalState.CANCEL_ACCEPTED
            )

    def commit_success(self, goal_handle) -> bool:
        return self._commit_active(goal_handle, _GoalTerminalState.SUCCEEDED)

    def commit_abort(self, goal_handle) -> bool:
        return self._commit_active(goal_handle, _GoalTerminalState.ABORTED)

    def commit_canceled(self, goal_handle) -> bool:
        with self._lock:
            if self._state_for(goal_handle) is not _GoalTerminalState.CANCEL_ACCEPTED:
                return False
            self._set_state(goal_handle, _GoalTerminalState.CANCELED)
            return True

    def tracked_goal_count(self) -> int:
        with self._lock:
            return len(self._states)

    def _commit_active(self, goal_handle, terminal_state: _GoalTerminalState) -> bool:
        with self._lock:
            state = self._state_for(goal_handle)
            if state is not _GoalTerminalState.ACTIVE:
                return False
            self._set_state(goal_handle, terminal_state)
            return True

    def _state_for(self, goal_handle) -> _GoalTerminalState:
        entry = self._states.get(id(goal_handle))
        if entry is None or entry[0]() is not goal_handle:
            return _GoalTerminalState.ACTIVE
        return entry[1]

    def _set_state(self, goal_handle, state: _GoalTerminalState) -> None:
        key = id(goal_handle)

        def discard(reference) -> None:
            with self._lock:
                entry = self._states.get(key)
                if entry is not None and entry[0] is reference:
                    del self._states[key]

        self._states[key] = (weakref.ref(goal_handle, discard), state)


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


def terminal_feedback(route: Sequence[int], finished_stages: int) -> StepFeedback:
    if finished_stages < 0 or finished_stages > len(route):
        raise ValueError(finished_stages)
    return StepFeedback(
        progress=finished_stages / len(route) if route else 1.0,
        current_tag=route[finished_stages - 1] if finished_stages else -1,
        next_tag=-1,
        finished_stages=finished_stages,
    )


def validate_step_delay(value: float) -> float:
    if value < 0.0:
        raise ValueError(f'step_delay_s must be non-negative, got {value}')
    return value
