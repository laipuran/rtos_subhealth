import type { TaskGoal, TaskRecord, CreateTaskResponse } from "../types/task"
import { parseError } from "./error"

const BASE = "/api/v1"

async function req<T>(method: string, path: string, body?: unknown): Promise<T> {
  const url = `${BASE}${path}`
  const opts: RequestInit = {
    method,
    headers: { "Content-Type": "application/json" },
    ...(body ? { body: JSON.stringify(body) } : {}),
  }

  console.groupCollapsed(
    `%c→%c ${method} ${path}`,
    "color:#4a9;font-weight:bold",
    "color:#888",
  )
  if (body) console.log("req:", body)
  const t0 = performance.now()
  const res = await fetch(url, opts)
  const dt = (performance.now() - t0).toFixed(0)
  const color = res.ok ? "color:#4a4" : "color:#a44"
  console.log(`%c← ${res.status} (${dt}ms)`, color)
  if (!res.ok) {
    try {
      const err = await res.clone().json()
      console.log("err:", err)
    } catch { /* ignore */ }
  }
  console.groupEnd()

  if (!res.ok) throw await parseError(res)
  return res.json()
}

export async function createTask(
  goal: TaskGoal,
  targetDevice = "",
): Promise<CreateTaskResponse> {
  return req("POST", "/tasks", { target_device: targetDevice, goal })
}

export async function listTasks(
  offset = 0,
  limit = 50,
): Promise<{ tasks: TaskRecord[]; total: number; offset: number; limit: number }> {
  return req("GET", `/tasks?offset=${offset}&limit=${limit}`)
}

export async function getTask(goalId: string): Promise<TaskRecord> {
  return req("GET", `/tasks/${goalId}`)
}

export async function cancelTask(goalId: string): Promise<void> {
  await req("POST", `/tasks/${goalId}/cancel`)
}
