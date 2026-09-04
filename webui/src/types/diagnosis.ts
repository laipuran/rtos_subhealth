export type DiagnosisSeverity = "normal" | "mild" | "moderate" | "severe" | "critical"

export type DiagnosisTrigger = "periodic" | "anomaly" | "manual"

export interface DiagnosisMetric {
  data_src: string
  data_type: string
  latest: number
  mean: number
  min: number
  max: number
  trend: string
  valid: boolean
}

export interface DiagnosisRecord {
  diagnosis_id: string
  source_ids: string[]
  trigger_type: DiagnosisTrigger
  severity: DiagnosisSeverity
  summary: string
  possible_causes: string[]
  recommendations: string[]
  confidence: number
  disclaimer: string
  raw_prompt: string
  error_code: string
  error_message: string
  metrics: DiagnosisMetric[]
  created_at: number
}

export interface DiagnosisListResponse {
  diagnoses: DiagnosisRecord[]
  total: number
  offset: number
  limit: number
}

export interface WsDiagnosisMessage {
  event?: string
  diagnosis_id: string
  trigger_type: DiagnosisTrigger
  severity: DiagnosisSeverity
  summary: string
  possible_causes: string[]
  recommendations: string[]
  confidence: number
  timestamp: number
  trace_id: string
  error_code?: string
  error_message?: string
  metrics?: DiagnosisMetric[]
}

export interface VitalsStreamMessage {
  event?: string
  timestamp: number
  metrics: DiagnosisMetric[]
}
