import sys
sys.path.insert(0, "ros2_ws/src/orchestration/exec_layer")

from exec_layer.fsm import ExecFSM


class TestExecFSM:
    def test_initial_state(self):
        fsm = ExecFSM()
        assert fsm.state == "idle"
        assert fsm.feedback_state == ""
        assert fsm.final_state == ""
        assert not fsm.is_active()
        assert not fsm.is_terminal()

    def test_full_lifecycle(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        assert fsm.state == "accepted"
        assert fsm.feedback_state == "accepted"

        fsm.start_plan()
        assert fsm.state == "planning"
        assert fsm.feedback_state == "running"

        fsm.plan_success()
        assert fsm.state == "moving"

        fsm.reach_tag()
        assert fsm.state == "approaching"

        fsm.rough_aligned()
        assert fsm.state == "aligning"

        fsm.aligned()
        assert fsm.state == "stabilizing"

        fsm.next_segment()
        assert fsm.state == "moving"

        fsm.all_done()
        assert fsm.state == "completed"
        assert fsm.final_state == "succeeded"
        assert fsm.is_terminal()

    def test_hold_flow(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.hold_position()
        assert fsm.state == "holding"
        assert fsm.feedback_state == "running"

        fsm.hold_done()
        assert fsm.state == "completed"
        assert fsm.final_state == "succeeded"

    def test_cancel_from_non_terminal(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.cancel()
        assert fsm.state == "canceled"
        assert fsm.final_state == "canceled"

    def test_cancel_from_terminal_ignored(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.hold_position()
        fsm.hold_done()
        assert fsm.state == "completed"

        fsm.cancel()
        assert fsm.state == "completed"

    def test_fail_flow(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.start_plan()
        fsm.plan_failed()
        assert fsm.state == "failed"
        assert fsm.final_state == "failed"
        assert fsm.is_terminal()

    def test_emergency_stop_resume(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.start_plan()
        fsm.plan_success()
        fsm.emergency_stop()
        assert fsm.state == "stopped"
        assert fsm.feedback_state == "stopped"

        fsm.stop_resolved()
        assert fsm.state == "moving"

    def test_emergency_stop_to_replan(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.start_plan()
        fsm.plan_success()
        fsm.emergency_stop()
        fsm.stop_replan()
        assert fsm.state == "replanning"

    def test_pause_resume(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.start_plan()
        fsm.plan_success()
        fsm.pause()
        assert fsm.state == "paused"
        assert fsm.feedback_state == "paused"
        assert fsm.is_active()

        fsm.resume()
        assert fsm.state == "moving"

    def test_pause_stop_symmetry(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.start_plan()
        fsm.plan_success()
        fsm.emergency_stop()
        fsm.pause()
        assert fsm.state == "paused"

        fsm.stop()
        assert fsm.state == "stopped"

    def test_replan_cycle(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.start_plan()
        fsm.plan_success()
        fsm.request_replan()
        assert fsm.state == "replanning"

        fsm.replan_success()
        assert fsm.state == "moving"

        fsm.request_replan()
        fsm.replan_failed()
        assert fsm.state == "failed"

    def test_is_executing_segment(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        assert not fsm.is_executing_segment()

        fsm.start_plan()
        fsm.plan_success()
        assert fsm.is_executing_segment()

        fsm.reach_tag()
        assert fsm.is_executing_segment()

        fsm.rough_aligned()
        assert fsm.is_executing_segment()

        fsm.aligned()
        assert fsm.is_executing_segment()

        fsm.all_done()
        assert not fsm.is_executing_segment()

    def test_all_done_from_approaching(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.start_plan()
        fsm.plan_success()
        fsm.reach_tag()
        fsm.all_done()
        assert fsm.state == "completed"

    def test_all_done_from_aligning(self):
        fsm = ExecFSM()
        fsm.accept_goal()
        fsm.start_plan()
        fsm.plan_success()
        fsm.reach_tag()
        fsm.rough_aligned()
        fsm.all_done()
        assert fsm.state == "completed"

    def test_feedback_and_final_state_mapping(self):
        fsm = ExecFSM()
        assert fsm.feedback_state == ""
        assert fsm.final_state == ""

        fsm.accept_goal()
        assert fsm.feedback_state == "accepted"

        fsm.start_plan()
        assert fsm.feedback_state == "running"

        fsm.plan_success()
        assert fsm.feedback_state == "running"

        fsm.pause()
        assert fsm.feedback_state == "paused"

        fsm.resume()
        fsm.emergency_stop()
        assert fsm.feedback_state == "stopped"

        fsm.cancel()
        assert fsm.final_state == "canceled"

    def test_phase_names(self):
        fsm = ExecFSM()
        assert fsm.phase == "空闲"
        fsm.accept_goal()
        assert fsm.phase == "任务接收"
        fsm.start_plan()
        fsm.plan_success()
        fsm.reach_tag()
        assert fsm.phase == "接近中"
