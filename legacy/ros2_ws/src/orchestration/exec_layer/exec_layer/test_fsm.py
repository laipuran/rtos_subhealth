from fsm import ExecFSM


def test_full_lifecycle():
    fsm = ExecFSM()
    assert fsm.state == "idle"

    fsm.accept_goal()
    assert fsm.state == "accepted"

    fsm.start_plan()
    assert fsm.state == "planning"

    fsm.plan_success()
    assert fsm.state == "moving"

    fsm.reach_tag()
    assert fsm.state == "approaching"

    fsm.rough_aligned()
    assert fsm.state == "aligning"

    fsm.aligned()
    assert fsm.state == "stabilizing"

    fsm.all_done()
    assert fsm.state == "completed"
    assert fsm.final_state == "succeeded"
    assert fsm.is_terminal()


def test_hold_path():
    fsm = ExecFSM()
    fsm.accept_goal()
    fsm.hold_position()
    assert fsm.state == "holding"

    fsm.hold_done()
    assert fsm.state == "completed"


def test_cancel_at_any_state():
    for state in ["accepted", "planning", "moving", "holding"]:
        fsm = ExecFSM()
        fsm.accept_goal()

        triggers = {
            "accepted": lambda f: None,
            "planning": lambda f: f.start_plan(),
            "moving": lambda f: (f.start_plan(), f.plan_success()),
            "holding": lambda f: f.hold_position(),
        }
        triggers[state](fsm)

        fsm.cancel()
        assert fsm.state == "canceled"
        assert fsm.is_terminal()


def test_replan():
    fsm = ExecFSM()
    fsm.accept_goal()
    fsm.start_plan()
    fsm.plan_success()  # -> moving

    fsm.request_replan()
    assert fsm.state == "replanning"

    fsm.replan_success()
    assert fsm.state == "moving"


def test_emergency_stop():
    fsm = ExecFSM()
    fsm.accept_goal()
    fsm.start_plan()
    fsm.plan_success()
    fsm.emergency_stop()
    assert fsm.state == "stopped"

    fsm.stop_resolved()
    assert fsm.state == "moving"


def test_pause_resume():
    fsm = ExecFSM()
    fsm.accept_goal()
    fsm.start_plan()
    fsm.plan_success()
    fsm.pause()
    assert fsm.state == "paused"

    fsm.resume()
    assert fsm.state == "moving"


def test_plan_failed():
    fsm = ExecFSM()
    fsm.accept_goal()
    fsm.start_plan()
    fsm.plan_failed()
    assert fsm.state == "failed"


def test_properties():
    fsm = ExecFSM()
    assert fsm.feedback_state == ""  # idle → no feedback
    assert fsm.phase == "空闲"

    fsm.accept_goal()
    assert fsm.feedback_state == "accepted"
    assert fsm.phase == "任务接收"
    assert not fsm.is_active()
    assert not fsm.is_terminal()

    fsm.start_plan()
    assert fsm.feedback_state == "running"


def test_ignore_invalid_transition():
    fsm = ExecFSM()
    # idle 状态下调用 plan_success 是非法转移，但设置了 ignore_invalid_triggers=True
    fsm.plan_success()  # 不会报错
    assert fsm.state == "idle"