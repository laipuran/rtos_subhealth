import { useEffect, useState } from "react"
import { getDiagnosis } from "../api/diagnostics"
import type { DiagnosisRecord } from "../types/diagnosis"
import DiagnosisSeverityBadge from "../components/DiagnosisSeverityBadge"

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
