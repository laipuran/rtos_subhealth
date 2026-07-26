import type { TagGraph } from "../types/map"
import { parseError } from "./error"

const BASE = "/api/v1"

export async function getMap(): Promise<TagGraph> {
  const res = await fetch(`${BASE}/map`)
  if (!res.ok) throw await parseError(res)
  return res.json()
}

export async function saveMap(data: TagGraph): Promise<{ status: string }> {
  const res = await fetch(`${BASE}/map`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  })
  const body = await res.json()
  if (!res.ok) throw await parseError(res)
  return body
}
