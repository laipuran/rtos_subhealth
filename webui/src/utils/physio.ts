import type { DiagnosisMetric } from "../types/diagnosis"

export interface PhysioMeta {
  label: string
  unit: string
  emoji: string
  color: string
}

// 常量顺序即图表/图例展示顺序
export const DATA_TYPE_ORDER: string[] = [
  "heart_rate",
  "spo2",
  "systolic_mmhg",
  "diastolic_mmhg",
  "body_temp_c",
  "respiratory_rate",
]

export const DATA_TYPE_META: Record<string, PhysioMeta> = {
  heart_rate: { label: "Heart", unit: "bpm", emoji: "❤️", color: "#ef4444" },
  spo2: { label: "SpO2", unit: "%", emoji: "🫁", color: "#10b981" },
  systolic_mmhg: { label: "Sys BP", unit: "mmHg", emoji: "🩸", color: "#3b82f6" },
  diastolic_mmhg: { label: "Dia BP", unit: "mmHg", emoji: "🩸", color: "#60a5fa" },
  body_temp_c: { label: "Temp", unit: "℃", emoji: "🌡️", color: "#f59e0b" },
  respiratory_rate: { label: "Resp", unit: "/min", emoji: "💨", color: "#8b5cf6" },
}

export function metaOf(data_type: string): PhysioMeta {
  return (
    DATA_TYPE_META[data_type] || {
      label: data_type,
      unit: "",
      emoji: "📉",
      color: "#9ca3af",
    }
  )
}

export function sortMetrics(metrics: DiagnosisMetric[]): DiagnosisMetric[] {
  const rank = (t: string) => {
    const idx = DATA_TYPE_ORDER.indexOf(t)
    return idx === -1 ? DATA_TYPE_ORDER.length : idx
  }
  return [...metrics].sort((a, b) => rank(a.data_type) - rank(b.data_type))
}
