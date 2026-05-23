import type { TaskFeedback, TaskGoal, TaskRecord, TaskResult } from "./task";

export type CreateTaskRequest = {
  target_device?: string;
  goal: TaskGoal;
};

export type CreateTaskResponse = {
  task_id: string;
  status: "success" | "error";
};

export type GetTaskResponse = TaskRecord;

export type ListTasksResponse = TaskRecord[];

export type CancelTaskResponse = {
  status: "success" | "error";
};

export type WsEvent =
  | { task_id: string; feedback: TaskFeedback }
  | { task_id: string; result: TaskResult };
