import type { WsEvent } from '../types/api';
import type { TaskRecord } from '../types/task';

export type TaskStoreState = {
    tasks: TaskRecord[];
    selectedTaskId?: string;
};

export function mergeTaskEvent(state: TaskStoreState, event: WsEvent): TaskStoreState {
    const existingIndex = state.tasks.findIndex((task) => task.task_id === event.task_id);
    if (existingIndex === -1) {
        return state;
    }

    const existingTask = state.tasks[existingIndex];
    const updatedTask: TaskRecord = { ...existingTask };

    if ('feedback' in event) {
        if (existingTask.feedback == null) {
            updatedTask.feedback = event.feedback;
        } else {
            const nextTimestamp = new Date(event.feedback.timestamp).getTime();
            const lastTimestamp = new Date(existingTask.feedback.timestamp).getTime();
            if (nextTimestamp > lastTimestamp) {
                updatedTask.feedback = event.feedback;
            }
        }
    }

    if ('result' in event) {
        if (existingTask.result == null) {
            updatedTask.result = event.result;
        }
    }

    if (updatedTask === existingTask) {
        return state;
    }

    const nextTasks = state.tasks.slice();
    nextTasks[existingIndex] = updatedTask;
    return {
        ...state,
        tasks: nextTasks,
    };
}

export function upsertTask(state: TaskStoreState, record: TaskRecord): TaskStoreState {
    const existingIndex = state.tasks.findIndex((task) => task.task_id === record.task_id);
    if (existingIndex === -1) {
        return {
            ...state,
            tasks: [...state.tasks, record],
        };
    }

    const existingTask = state.tasks[existingIndex];
    const updatedTask: TaskRecord = {
        ...existingTask,
        goal: record.goal,
    };

    if (record.feedback != null) {
        if (existingTask.feedback == null) {
            updatedTask.feedback = record.feedback;
        } else {
            const nextTimestamp = new Date(record.feedback.timestamp).getTime();
            const lastTimestamp = new Date(existingTask.feedback.timestamp).getTime();
            if (nextTimestamp > lastTimestamp) {
                updatedTask.feedback = record.feedback;
            }
        }
    }

    if (existingTask.result == null && record.result != null) {
        updatedTask.result = record.result;
    }

    const nextTasks = state.tasks.slice();
    nextTasks[existingIndex] = updatedTask;
    return {
        ...state,
        tasks: nextTasks,
    };
}

export function selectTask(state: TaskStoreState, taskId?: string): TaskStoreState {
    if (state.selectedTaskId === taskId) {
        return state;
    }

    return {
        ...state,
        selectedTaskId: taskId,
    };
}
