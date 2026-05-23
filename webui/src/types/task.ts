export type TaskType = 'patrol' | 'navigate' | 'hold';

// accepted: 任务已被接受，但尚未开始执行
// running: 任务正在执行中
// paused: 任务被手动暂停
// stopped: 任务因成功或者失败而停止
export type TaskState = 'accepted' | 'running' | 'paused' | 'stopped';

// success: 任务成功完成
// failed: 任务执行失败
// cancelled: 任务被取消
export type TaskFinalState = 'succeeded' | 'failed' | 'canceled';

export type TaskGoal = {
    type:         TaskType;
    priority:     number;
    route?:       number[];
    target_tags?: number[];
    constraints?: TaskConstraints;
    deadline?:    number;
    timestamp:    string;
};

export type TaskFeedback = {
    state:           TaskState;
    route?:          number[];
    finished_stages: number;
    error_code?:     TaskError;
    result?:         TaskResult;
    timestamp:       string;
};

export type TaskError =
    | 'NoErr'
    | 'InvalidGoal'
    | 'GraphMissing'
    | 'UnknownTag'
    | 'Unreachable'
    | 'ConstraintViolation'
    | 'InternalError'
    | 'Timeout';

export type TaskConstraints = Record<string, unknown>;

export type TaskResult = {
    final_state: TaskFinalState;
    error_code?: TaskError;
    message?:    string;
    timestamp:   string;
};

export type TaskRecord = {
    task_id:   string;
    goal:      TaskGoal;
    feedback?: TaskFeedback;
    result?:   TaskResult;
};
