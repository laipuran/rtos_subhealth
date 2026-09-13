from transitions import Machine


class ExecFSM:
    """执行层状态机 —— 基于 transitions 库

    覆盖 RFC-003 (决策层任务流) + RFC-004 (规划接口) + RFC-002 (姿态调整).

    ── 状态总览 ──────────────────────────────────────────────
    生命周期:  idle → accepted → planning
    段执行:    moving → approaching → aligning → stabilizing
    等待驻留:  holding                          ← hold 类型任务专用
    异常:      replanning / stopped
    外部控制:  paused / canceled
    终态:      completed / failed / canceled

    ── 与 RFC 反馈态 / 终态的映射 ──────────────────────────────
    feedback.state:  accepted | running | paused | stopped
    result.final_state:  succeeded | failed | canceled
    """

    states = [
        "idle",
        "accepted",
        "planning",
        "moving",
        "approaching",
        "aligning",
        "stabilizing",
        "holding",
        "replanning",
        "stopped",
        "paused",
        "completed",
        "failed",
        "canceled",
    ]

    transitions = [
        # ── 任务生命周期 ──────────────────────────────────────
        {"trigger": "accept_goal",    "source": "idle",          "dest": "accepted"},
        {"trigger": "start_plan",     "source": "accepted",      "dest": "planning"},
        {"trigger": "plan_success",   "source": "planning",      "dest": "moving"},
        {"trigger": "plan_failed",    "source": "planning",      "dest": "failed"},

        # ── hold 类型: 跳过规划，直接驻留等待 ─────────────────
        {"trigger": "hold_position",  "source": "accepted",      "dest": "holding"},

        # ── 段内执行阶段推进 ──────────────────────────────────
        {"trigger": "reach_tag",      "source": "moving",        "dest": "approaching"},
        {"trigger": "rough_aligned",  "source": "approaching",   "dest": "aligning"},
        {"trigger": "aligned",        "source": "aligning",      "dest": "stabilizing"},

        # ── 段完成 / 全部完成 ─────────────────────────────────
        {"trigger": "next_segment",   "source": "stabilizing",   "dest": "moving"},
        {"trigger": "all_done",       "source": "stabilizing",   "dest": "completed"},

        # ── hold 退出 ─────────────────────────────────────────
        {"trigger": "hold_done",      "source": "holding",       "dest": "completed"},

        # ── 重规划 ────────────────────────────────────────────
        {"trigger": "request_replan", "source": [
            "moving", "approaching", "aligning", "stabilizing"],
         "dest": "replanning"},
        {"trigger": "replan_success", "source": "replanning",     "dest": "moving"},
        {"trigger": "replan_failed",  "source": "replanning",     "dest": "failed"},

        # ── 紧急停止 ──────────────────────────────────────────
        {"trigger": "emergency_stop", "source": [
            "moving", "approaching", "aligning", "stabilizing",
            "holding", "replanning"],
         "dest": "stopped"},
        {"trigger": "stop_resolved",  "source": "stopped",       "dest": "moving"},
        {"trigger": "stop_replan",    "source": "stopped",       "dest": "replanning"},

        # ── 暂停 / 恢复 ───────────────────────────────────────
        {"trigger": "pause",          "source": [
            "moving", "approaching", "aligning", "stabilizing",
            "holding", "replanning", "stopped"],
         "dest": "paused"},
        {"trigger": "resume",         "source": "paused",        "dest": "moving"},

        # ── 终态 (从所有非终态均可进入) ─────────────────────
        {"trigger": "cancel",         "source": "*",             "dest": "canceled"},
        {"trigger": "fail",           "source": [
            "accepted", "planning", "moving", "approaching",
            "aligning", "stabilizing", "holding", "replanning",
            "stopped", "paused"],
         "dest": "failed"},
    ]

    # ── 查询接口 ──────────────────────────────────────────────

    _FEEDBACK_MAP = {
        "accepted":    "accepted",
        "planning":    "running",
        "moving":      "running",
        "approaching": "running",
        "aligning":    "running",
        "stabilizing": "running",
        "holding":     "running",
        "replanning":  "running",
        "stopped":     "stopped",
        "paused":      "paused",
    }

    _EXECUTING_STATES = {
        "planning", "moving", "approaching",
        "aligning", "stabilizing", "holding",
        "replanning", "stopped",
    }

    def __init__(self):
        self.error_code = ""
        self.message = ""
        self.machine = Machine(
            model=self,
            states=self.states,
            transitions=self.transitions,
            initial="idle",
            ignore_invalid_triggers=True,
            after_state_change=self._log_transition,
        )

    @property
    def feedback_state(self) -> str:
        """RFC-003 feedback.state: accepted / running / paused / stopped"""
        return self._FEEDBACK_MAP.get(self.state, "")

    @property
    def final_state(self) -> str:
        """RFC-003 result.final_state: succeeded / failed / canceled"""
        if self.state == "completed":
            return "succeeded"
        if self.state in ("failed", "canceled"):
            return self.state
        return ""

    @property
    def phase(self) -> str:
        _names = {
            "idle":        "空闲",
            "accepted":    "任务接收",
            "planning":    "规划中",
            "moving":      "行进中",
            "approaching": "接近中",
            "aligning":    "对正中",
            "stabilizing": "稳定中",
            "holding":     "驻留等待",
            "replanning":  "重规划",
            "stopped":     "已停止",
            "paused":      "已暂停",
            "completed":   "已完成",
            "failed":      "失败",
            "canceled":    "已取消",
        }
        return _names.get(self.state, self.state)

    def is_active(self) -> bool:
        return self.state in self._EXECUTING_STATES

    def is_executing_segment(self) -> bool:
        return self.state in {
            "moving", "approaching", "aligning", "stabilizing",
        }

    def is_terminal(self) -> bool:
        return self.state in ("completed", "failed", "canceled")

    # ── 回调 ──────────────────────────────────────────────────

    def _log_transition(self) -> None:
        pass
