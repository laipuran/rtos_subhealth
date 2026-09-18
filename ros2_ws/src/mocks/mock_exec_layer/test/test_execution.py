from dataclasses import FrozenInstanceError

import pytest

from mock_exec_layer.execution import (
    canceled_feedback,
    execution_steps,
    validate_step_delay,
)


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
