string task_id
string type              # go_to_tag | patrol_route | hold | cancel | resume
int32 priority
string route_id
int32[] target_tags
int32 start_tag          # -1 表示自动推断起点；>=0 表示显式起点
string target_task_id    # cancel/resume 必填：被取消/恢复的任务 task_id
rcl_interfaces/Parameter[] constraints
int64 deadline_ms        # 相对 issue_time 的超时(ms)，0 表示不设截止
builtin_interfaces/Time issue_time
