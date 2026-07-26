export interface ApiError {
  code: string
  message: string
  details?: any
}

export async function parseError(res: Response): Promise<Error> {
  try {
    const body = await res.json()
    const err = body?.error
    if (err?.code && err?.message) {
      return new Error(`[${err.code}] ${err.message}${err.details ? " " + JSON.stringify(err.details) : ""}`)
    }
    return new Error(err?.message || res.statusText || `HTTP ${res.status}`)
  } catch {
    return new Error(res.statusText || `HTTP ${res.status}`)
  }
}
