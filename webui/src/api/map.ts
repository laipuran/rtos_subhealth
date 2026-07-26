import type { TagGraph } from "../types/map"

const BASE = "/api/v1"

async function req<T>(method: string, path: string, body?: unknown): Promise<T> {
  const url = `${BASE}${path}`
  const opts: RequestInit = {
    method,
    headers: { "Content-Type": "application/json" },
    ...(body ? { body: JSON.stringify(body) } : {}),
  }

  console.groupCollapsed(`%c→%c ${method} ${path}`, "color:#4a9;font-weight:bold", "color:#888")
  const t0 = performance.now()
  const res = await fetch(url, opts)
  const dt = (performance.now() - t0).toFixed(0)
  const color = res.ok ? "color:#4a4" : "color:#a44"
  console.log(`%c← ${res.status} (${dt}ms)`, color)
  console.groupEnd()

  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new Error(body?.error?.message || res.statusText)
  }
  return res.json()
}

export async function getMap(): Promise<TagGraph> {
  return req("GET", "/map")
}

export async function saveMap(data: TagGraph): Promise<{ status: string }> {
  return req("PUT", "/map", data)
}
