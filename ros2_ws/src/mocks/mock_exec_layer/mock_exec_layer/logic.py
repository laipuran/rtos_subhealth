"""Pure deterministic behavior for the mock execution layer."""

from dataclasses import dataclass
from enum import Enum, auto
import json
import threading
import weakref


class UnsupportedPrimitive(ValueError):
    """Raised when a primitive is not supported by the mock."""


class InvalidPayload(ValueError):
    """Raised when a primitive payload violates its schema."""


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
class ExecutionStep:
    progress: float
    phase: str
    details: dict


def parse_payload(primitive: str, payload_json: str) -> dict:
    try:
        payload = json.loads(payload_json)
    except (TypeError, json.JSONDecodeError) as error:
        raise InvalidPayload('payload_json must contain valid JSON') from error

    if not isinstance(payload, dict):
        raise InvalidPayload('payload must be a JSON object')

    if primitive == 'hold':
        if payload:
            raise InvalidPayload('hold payload must be empty')
        return payload

    if primitive == 'go_to_tag':
        if set(payload) != {'target_tag'}:
            raise InvalidPayload('go_to_tag payload requires only target_tag')
        target_tag = payload['target_tag']
        if isinstance(target_tag, bool) or not isinstance(target_tag, int):
            raise InvalidPayload('target_tag must be an integer')
        if target_tag < -(2**31) or target_tag > 2**31 - 1:
            raise InvalidPayload('target_tag must fit in a signed 32-bit integer')
        return payload

    raise UnsupportedPrimitive(primitive)


def execution_steps(primitive: str, payload: dict) -> tuple[ExecutionStep, ...]:
    if primitive == 'hold':
        return (ExecutionStep(progress=1.0, phase='holding', details={}),)

    if primitive == 'go_to_tag':
        target_tag = payload['target_tag']
        return tuple(
            ExecutionStep(
                progress=index / 3,
                phase='moving_to_tag',
                details={
                    'current_tag': target_tag if index == 3 else -1,
                    'next_tag': -1 if index == 3 else target_tag,
                },
            )
            for index in range(1, 4)
        )

    raise UnsupportedPrimitive(primitive)


def canceled_feedback(last_step: ExecutionStep | None) -> ExecutionStep:
    details = dict(last_step.details) if last_step is not None else {}
    if 'next_tag' in details:
        details['next_tag'] = -1
    return ExecutionStep(
        progress=last_step.progress if last_step is not None else 0.0,
        phase='canceled',
        details=details,
    )


def validate_step_delay(value: float) -> float:
    if value < 0.0:
        raise ValueError(f'step_delay_s must be non-negative, got {value}')
    return value
