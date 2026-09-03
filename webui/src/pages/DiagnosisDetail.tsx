import { useEffect, useState } from "react"
import { getDiagnosis } from "../api/diagnostics"
import type { DiagnosisMetric, DiagnosisRecord } from "../types/diagnosis"
import DiagnosisSeverityBadge from "../components/DiagnosisSeverityBadge"
import { sortMetrics, metaOf } from "../utils/physio"

interface Props {
  diagnosisId: string | null
  onBack: () => void
  liveUpdates: Record<string, Partial<DiagnosisRecord>>
}

export default function DiagnosisDetail({ diagnosisId, onBack, liveUpdates }: Props) {
  const [rec, setRec] = useState<DiagnosisRecord | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!diagnosisId) return
    setLoading(true)
    getDiagnosis(diagnosisId)
      .then(setRec)
      .catch(() => setRec(null))
      .finally(() => setLoading(false))
  }, [diagnosisId])

  const merged = diagnosisId && liveUpdates[diagnosisId] && rec
    ? { ...rec, ...liveUpdates[diagnosisId] }
    : rec

  if (!diagnosisId) return null
  if (loading) return <p className="text-gray-400 text-sm">Loading...</p>
  if (!merged) return <p className="text-red-500 text-sm">Diagnosis not found.</p>

  return (
    <div className="space-y-4">
      <button onClick={onBack} className="text-sm text-blue-600 hover:underline">
        &larr; Back
      </button>

      <div className="flex items-center justify-between">
        <h2 className="text-lg font-bold">Diagnosis Detail</h2>
        <DiagnosisSeverityBadge severity={merged.severity} />
      </div>

      <table className="w-full text-sm">
        <tbody>
          {[
            ["ID", merged.diagnosis_id],
            ["Trigger", merged.trigger_type],
            ["Sources", merged.source_ids.join(", ") || "-"],
            ["Confidence", merged.confidence != null ? merged.confidence.toFixed(2) : "-"],
            ["Summary", merged.summary || "-"],
            ["Error", merged.error_code ? `${merged.error_code}: ${merged.error_message}` : "-"],
          ].map(([label, val]) => (
            <tr key={label} className="border-b">
              <td className="py-1 pr-4 text-gray-500 font-medium">{label}</td>
              <td className="py-1">{val}</td>
            </tr>
          ))}
        </tbody>
      </table>

      {(merged.metrics?.length ?? 0) > 0 && (
        <div>
          <h3 className="font-medium text-gray-700">采集指标</h3>
          <MetricTable metrics={merged.metrics} />
        </div>
      )}

      <div>
        <h3 className="font-medium text-gray-700">Possible Causes</h3>
        {merged.possible_causes?.length ? (
          <ul className="list-disc pl-5 text-sm text-gray-600">
            {merged.possible_causes.map((c, i) => <li key={i}>{c}</li>)}
          </ul>
        ) : <p className="text-sm text-gray-400">-</p>}
      </div>

      <div>
        <h3 className="font-medium text-gray-700">Recommendations</h3>
        {merged.recommendations?.length ? (
          <ul className="list-disc pl-5 text-sm text-gray-600">
            {merged.recommendations.map((c, i) => <li key={i}>{c}</li>)}
          </ul>
        ) : <p className="text-sm text-gray-400">-</p>}
      </div>

      {merged.disclaimer && (
        <p className="text-xs text-gray-400 italic">{merged.disclaimer}</p>
      )}
    </div>
  )
}

function MetricTable({ metrics }: { metrics: DiagnosisMetric[] }) {
  const rows = sortMetrics(metrics)
  const fmt = (v: number) => (v == null ? "-" : Number(v).toFixed(1))
  const fmtTrend = (t: string) => {
    const map: Record<string, string> = {
      increasing: "上升",
      decreasing: "下降",
      stable: "平稳",
      unknown: "未知",
    }
    return map[t] || t
  }

  return (
    <table className="w-full text-sm border">
      <thead>
        <tr className="bg-gray-50 text-left">
          <th className="px-2 py-1 font-medium text-gray-600">数据</th>
          <th className="px-2 py-1 font-medium text-gray-600 text-right">最新</th>
          <th className="px-2 py-1 font-medium text-gray-600 text-right">均值</th>
          <th className="px-2 py-1 font-medium text-gray-600 text-right">最小</th>
          <th className="px-2 py-1 font-medium text-gray-600 text-right">最大</th>
          <th className="px-2 py-1 font-medium text-gray-600 text-right">趋势</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((m) => {
          const meta = metaOf(m.data_type)
          return (
            <tr key={m.data_src || m.data_type} className="border-t">
              <td className="px-2 py-1 text-gray-700">
                {m.data_type}（{meta.label}）
              </td>
              <td className="px-2 py-1 text-right font-mono">{fmt(m.latest)}{meta.unit}</td>
              <td className="px-2 py-1 text-right font-mono">{fmt(m.mean)}{meta.unit}</td>
              <td className="px-2 py-1 text-right font-mono">{fmt(m.min)}{meta.unit}</td>
              <td className="px-2 py-1 text-right font-mono">{fmt(m.max)}{meta.unit}</td>
              <td className="px-2 py-1 text-right text-gray-600">{fmtTrend(m.trend)}</td>
            </tr>
          )
        })}
      </tbody>
    </table>
  )
}
