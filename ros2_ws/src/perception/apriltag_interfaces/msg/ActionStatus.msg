string task_id           # 全局唯一任务 ID
string state             # accepted|running|succeeded|failed|canceled|paused
float32 progress         # 0.0-1.0 任务进度
int32 current_tag        # 当前识别/对齐的 tag
int32 next_tag           # 计划到达的下一个 tag
int32 goal_tag           # 本任务最终目标 tag（未知填 -1）
int32 total_steps        # 总步骤数（未知填 0）
int32 completed_steps    # 已完成步骤数
string last_move_id      # 最近完成的执行步骤 ID
string error_code        # 机器可读错误码
string message           # 执行层生成的可读说明，供 UI/日志/断言
builtin_interfaces/Time timestamp
