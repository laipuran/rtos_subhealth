export type TaskType = "go_to_tag"

export type TaskState =
  | "accepted"
  | "running"
  | "succeeded"
  | "failed"


export interface TaskGoal {
  type: TaskType
  target_tags?: number[]
  deadline_ms?: number
}

export interface TaskRecord {
  goal_id: string
  type: TaskType
  priority: number
  route_id: string
  target_tags: number[]
  state: TaskState
  progress: number
  current_tag: number
  next_tag: number
  error_code: string
  message: string
  final_state: string | null
  route: number[]
  finished_stages: number
  created_at: number
  updated_at: number
}

export interface WsMessage {
  goal_id: string
  event: "feedback" | "result"
  state?: TaskState
  progress?: number
  current_tag?: number
  next_tag?: number
  error_code?: string
  message?: string
  final_state?: TaskState
  route?: number[]
  finished_stages?: number
}

export interface CreateTaskResponse {
  task_id: string
  status: string
  type: TaskType
}
