"""Thread-safe terminal-state coordination for ROS action goals."""

from enum import Enum, auto
import threading
import weakref


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
