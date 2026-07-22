import type { TaskGoal, TaskRecord, CreateTaskResponse } from "../types/task"

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
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(err.error || "create task failed")
  }
  return res.json()
}

export async function listTasks(): Promise<{ tasks: TaskRecord[] }> {
  const res = await fetch(`${BASE}/tasks`)
  if (!res.ok) throw new Error("list tasks failed")
  return res.json()
}

export async function getTask(goalId: string): Promise<TaskRecord> {
  const res = await fetch(`${BASE}/tasks/${goalId}`)
  if (!res.ok) throw new Error("task not found")
  return res.json()
}

export async function cancelTask(goalId: string): Promise<void> {
  const res = await fetch(`${BASE}/tasks/${goalId}/cancel`, { method: "POST" })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(err.error || "cancel failed")
  }
}
