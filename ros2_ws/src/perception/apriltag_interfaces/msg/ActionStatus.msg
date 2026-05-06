string task_id
string state            # accepted|running|succeeded|failed|canceled|paused
float32 progress
int32 current_tag
int32 next_tag
int32 goal_tag           # 本任务最终目标 tag（未知填 -1）
int32 total_steps        # 原子移动总步数（未知填 0）
int32 completed_steps    # 已完成原子移动步数
string last_move_id     # optional: 最近完成的原子移动 ID
string error_code       # 建议使用标准错误码枚举
string message
builtin_interfaces/Time timestamp
