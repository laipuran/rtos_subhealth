import type { TagGraph } from "../types/map"

const BASE = "/api/v1"

export async function getMap(): Promise<TagGraph> {
  const res = await fetch(`${BASE}/map`)
  if (!res.ok) throw new Error("load map failed")
  return res.json()
}

export async function saveMap(data: TagGraph): Promise<{ status: string }> {
  const res = await fetch(`${BASE}/map`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  })
  const body = await res.json()
  if (!res.ok) {
    throw new Error(body.error || "save map failed")
  }
  return body
}
