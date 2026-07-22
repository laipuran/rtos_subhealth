import type { TaskGoal, TaskRecord, CreateTaskResponse } from "../types/task"
import { parseError } from "./error"

const BASE = "/api/v1"

export async function createTask(
  goal: TaskGoal,
  targetDevice = "",
): Promise<CreateTaskResponse> {
  const res = await fetch(`${BASE}/tasks`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ target_device: targetDevice, goal }),
  })
  if (!res.ok) throw await parseError(res)
  return res.json()
}

export async function listTasks(
  offset = 0,
  limit = 50,
): Promise<{ tasks: TaskRecord[]; total: number; offset: number; limit: number }> {
  const res = await fetch(`${BASE}/tasks?offset=${offset}&limit=${limit}`)
  if (!res.ok) throw await parseError(res)
  return res.json()
}

export async function getTask(goalId: string): Promise<TaskRecord> {
  const res = await fetch(`${BASE}/tasks/${goalId}`)
  if (!res.ok) throw await parseError(res)
  return res.json()
}

export async function cancelTask(goalId: string): Promise<void> {
  const res = await fetch(`${BASE}/tasks/${goalId}/cancel`, { method: "POST" })
  if (!res.ok) throw await parseError(res)
}
