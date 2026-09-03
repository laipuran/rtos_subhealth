import type { DiagnosisRecord, DiagnosisListResponse } from "../types/diagnosis"
import { parseError } from "./error"

const BASE = "/api/v1"

export async function listDiagnoses(
  offset = 0,
  limit = 50,
): Promise<DiagnosisListResponse> {
  const res = await fetch(`${BASE}/diagnostics?offset=${offset}&limit=${limit}`)
  if (!res.ok) throw await parseError(res)
  return res.json()
}

export async function getDiagnosis(id: string): Promise<DiagnosisRecord> {
  const res = await fetch(`${BASE}/diagnostics/${id}`)
  if (!res.ok) throw await parseError(res)
  return res.json()
}

export async function triggerDiagnosis(): Promise<{ status: string; trigger_type: string }> {
  const res = await fetch(`${BASE}/diagnostics`, { method: "POST" })
  if (!res.ok) throw await parseError(res)
  return res.json()
}
