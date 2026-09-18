from dataclasses import FrozenInstanceError
import gc
import threading
import weakref

import pytest

from mock_exec_layer.logic import (
    canceled_feedback,
    execution_steps,
    GoalTerminalCoordinator,
    InvalidPayload,
    parse_payload,
    UnsupportedPrimitive,
    validate_step_delay,
)


def test_hold_payload_is_empty_object():
    assert parse_payload('hold', '{}') == {}


@pytest.mark.parametrize('payload', ['[]', '{"unexpected":1}', 'not-json'])
def test_hold_rejects_invalid_payload(payload):
    with pytest.raises(InvalidPayload):
        parse_payload('hold', payload)


def test_go_to_tag_payload_contains_one_signed_32_bit_integer():
    assert parse_payload('go_to_tag', '{"target_tag":42}') == {'target_tag': 42}
    assert parse_payload('go_to_tag', '{"target_tag":-1}') == {'target_tag': -1}


@pytest.mark.parametrize(
    'payload',
    [
        '{}',
        '{"target_tag":true}',
        '{"target_tag":2147483648}',
        '{"target_tag":-2147483649}',
        '{"target_tag":1,"extra":2}',
    ],
)
def test_go_to_tag_rejects_invalid_payload(payload):
    with pytest.raises(InvalidPayload):
        parse_payload('go_to_tag', payload)


def test_unknown_primitive_is_rejected():
    with pytest.raises(UnsupportedPrimitive):
        parse_payload('dance', '{}')


def test_hold_has_one_complete_feedback_step():
    steps = execution_steps('hold', {})
    assert len(steps) == 1
    assert steps[0].progress == 1.0
    assert steps[0].phase == 'holding'
    assert steps[0].details == {}


def test_go_to_tag_has_deterministic_progress_and_terminal_details():
    steps = execution_steps('go_to_tag', {'target_tag': 42})
    assert [step.progress for step in steps] == pytest.approx([1 / 3, 2 / 3, 1.0])
    assert all(step.phase == 'moving_to_tag' for step in steps)
    assert steps[0].details == {'current_tag': -1, 'next_tag': 42}
    assert steps[-1].details == {'current_tag': 42, 'next_tag': -1}


def test_canceled_feedback_preserves_progress_and_clears_next_tag():
    running = execution_steps('go_to_tag', {'target_tag': 42})[0]
    canceled = canceled_feedback(running)
    assert canceled.progress == running.progress
    assert canceled.phase == 'canceled'
    assert canceled.details == {'current_tag': -1, 'next_tag': -1}


def test_feedback_is_immutable():
    feedback = execution_steps('hold', {})[0]
    with pytest.raises(FrozenInstanceError):
        feedback.progress = 0.5


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
