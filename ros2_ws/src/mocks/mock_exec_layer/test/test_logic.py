from dataclasses import FrozenInstanceError
import gc
import threading
import weakref

from mock_exec_layer.logic import (
    GoalTerminalCoordinator,
    route_for,
    step_feedback,
    terminal_feedback,
    UnsupportedTask,
    validate_step_delay,
)

import pytest


def test_hold_has_no_route():
    assert route_for('hold', [1]) == []


def test_go_to_tag_uses_first_target():
    assert route_for('go_to_tag', [7, 8]) == [7]


def test_patrol_uses_all_targets():
    assert route_for('patrol_route', [7, 8]) == [7, 8]


def test_unknown_task_is_rejected():
    with pytest.raises(UnsupportedTask):
        route_for('dance', [])


def test_final_step_clears_next_tag():
    feedback = step_feedback([7, 8], 1)
    assert feedback.progress == 1.0
    assert feedback.current_tag == 8
    assert feedback.next_tag == -1


def test_go_to_tag_uses_fallback_without_targets():
    assert route_for('go_to_tag', []) == [42]


def test_patrol_uses_fallback_without_targets():
    assert route_for('patrol_route', []) == [10, 20, 30]


def test_feedback_reports_completed_stage_and_next_tag():
    feedback = step_feedback([7, 8], 0)
    assert feedback.progress == 0.5
    assert feedback.current_tag == 7
    assert feedback.next_tag == 8
    assert feedback.finished_stages == 1


@pytest.mark.parametrize('index', [-1, 2])
def test_feedback_rejects_invalid_index(index):
    with pytest.raises(IndexError):
        step_feedback([7, 8], index)


def test_feedback_is_immutable():
    feedback = step_feedback([7], 0)
    with pytest.raises(FrozenInstanceError):
        feedback.progress = 0.5


def test_hold_terminal_feedback_is_complete_and_has_no_next_tag():
    feedback = terminal_feedback([], 0)
    assert feedback.progress == 1.0
    assert feedback.current_tag == -1
    assert feedback.next_tag == -1
    assert feedback.finished_stages == 0


def test_canceled_terminal_feedback_preserves_completed_progress():
    feedback = terminal_feedback([7, 8], 1)
    assert feedback.progress == 0.5
    assert feedback.current_tag == 7
    assert feedback.next_tag == -1
    assert feedback.finished_stages == 1


def test_step_delay_must_be_non_negative():
    with pytest.raises(ValueError, match='step_delay_s must be non-negative'):
        validate_step_delay(-0.1)


@pytest.mark.parametrize('value', [0.0, 0.2])
def test_non_negative_step_delay_is_accepted(value):
    assert validate_step_delay(value) == value


class Goal:
    pass


def test_cancel_claim_prevents_success_commit():
    coordinator = GoalTerminalCoordinator()
    goal = Goal()

    assert coordinator.request_cancel(goal)
    assert coordinator.is_cancel_pending(goal)
    assert not coordinator.commit_success(goal)


def test_cancel_claim_prevents_abort_and_commits_canceled():
    coordinator = GoalTerminalCoordinator()
    goal = Goal()

    assert coordinator.request_cancel(goal)
    assert not coordinator.commit_abort(goal)
    assert coordinator.commit_canceled(goal)
    assert not coordinator.request_cancel(goal)


def test_success_commit_rejects_later_cancel():
    coordinator = GoalTerminalCoordinator()
    goal = Goal()

    assert coordinator.commit_success(goal)
    assert not coordinator.request_cancel(goal)


def test_terminal_claim_is_atomic_under_concurrent_requests():
    for _ in range(100):
        coordinator = GoalTerminalCoordinator()
        goal = Goal()
        barrier = threading.Barrier(3)
        outcomes = {}

        def cancel():
            barrier.wait()
            outcomes['cancel'] = coordinator.request_cancel(goal)

        def succeed():
            barrier.wait()
            outcomes['success'] = coordinator.commit_success(goal)

        cancel_thread = threading.Thread(target=cancel)
        success_thread = threading.Thread(target=succeed)
        cancel_thread.start()
        success_thread.start()
        barrier.wait()
        cancel_thread.join()
        success_thread.join()

        assert outcomes in (
            {'cancel': True, 'success': False},
            {'cancel': False, 'success': True},
        )


def test_different_goals_have_independent_terminal_states():
    coordinator = GoalTerminalCoordinator()
    canceled_goal = Goal()
    successful_goal = Goal()

    assert coordinator.request_cancel(canceled_goal)
    assert coordinator.commit_success(successful_goal)
    assert coordinator.is_cancel_pending(canceled_goal)
    assert not coordinator.is_cancel_pending(successful_goal)


def test_goal_state_does_not_outlive_goal_handle():
    coordinator = GoalTerminalCoordinator()
    goal = Goal()
    goal_reference = weakref.ref(goal)
    assert coordinator.commit_success(goal)
    assert coordinator.tracked_goal_count() == 1

    del goal
    gc.collect()

    assert goal_reference() is None
    assert coordinator.tracked_goal_count() == 0
