import { createContext, useContext, useState, type ReactNode } from "react"
import { useVitalsWS } from "../hooks/useVitalsWS"
import type { DiagnosisMetric } from "../types/diagnosis"

interface RawPoint {
  t: number
  raw: Record<string, number>
}

interface VitalsCtx {
  points: RawPoint[]
}

const MAX_POINTS = 60 // 1 Hz → 最近 ~60s

const Ctx = createContext<VitalsCtx>({ points: [] })

export function VitalsProvider({ children }: { children: ReactNode }) {
  const [points, setPoints] = useState<RawPoint[]>([])

  useVitalsWS((msg) => {
    const t = msg.timestamp
    const raw: Record<string, number> = {}
    msg.metrics.forEach((m: DiagnosisMetric) => {
      raw[m.data_type] = m.latest
    })
    setPoints((prev) => {
      const last = prev[prev.length - 1]
      if (last && Math.abs(last.t - t) < 0.5) {
        return [...prev.slice(0, -1), { t, raw }]
      }
      const next = [...prev, { t, raw }]
      return next.length > MAX_POINTS ? next.slice(next.length - MAX_POINTS) : next
    })
  })

  return <Ctx.Provider value={{ points }}>{children}</Ctx.Provider>
}

export function useVitals(): VitalsCtx {
  return useContext(Ctx)
}
