from dataclasses import FrozenInstanceError

from mock_exec_layer.logic import route_for, step_feedback, UnsupportedTask

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
