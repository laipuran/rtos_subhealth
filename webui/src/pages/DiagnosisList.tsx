import { useEffect, useState } from "react"
import { listDiagnoses, triggerDiagnosis } from "../api/diagnostics"
import type { DiagnosisRecord } from "../types/diagnosis"
import DiagnosisSeverityBadge from "../components/DiagnosisSeverityBadge"
import { useToast } from "../components/Toast"

interface Props {
  refreshKey: number
  onSelect: (id: string) => void
  liveUpdates: Record<string, Partial<DiagnosisRecord>>
}

const PAGE_SIZE = 20

export default function DiagnosisList({ refreshKey, onSelect, liveUpdates }: Props) {
  const { toast } = useToast()
  const [items, setItems] = useState<DiagnosisRecord[]>([])
  const [total, setTotal] = useState(0)
  const [offset, setOffset] = useState(0)
  const [loading, setLoading] = useState(true)
  const [triggering, setTriggering] = useState(false)

  useEffect(() => {
    setOffset(0)
    setItems([])
    setLoading(true)
    listDiagnoses(0, PAGE_SIZE)
      .then((data) => {
        setItems(data.diagnoses)
        setTotal(data.total)
      })
      .catch((err: any) => {
        toast(err?.message || "Failed to load diagnoses", "error")
      })
      .finally(() => setLoading(false))
  }, [refreshKey])

  const loadMore = async () => {
    const newOffset = offset + PAGE_SIZE
    try {
      const data = await listDiagnoses(newOffset, PAGE_SIZE)
      setItems((prev) => [...prev, ...data.diagnoses])
      setOffset(newOffset)
    } catch (err: any) {
      toast(err?.message || "Failed to load more", "error")
    }
  }

  const merged = items.map((d) => {
    const upd = liveUpdates[d.diagnosis_id]
    return upd ? { ...d, ...upd } : d
  })

  const handleTrigger = async () => {
    setTriggering(true)
    try {
      await triggerDiagnosis()
      toast("Manual diagnosis triggered", "success")
    } catch (err: any) {
      toast(err.message || "trigger failed", "error")
    }
    setTriggering(false)
  }

  if (loading) return <p className="text-gray-400 text-sm">Loading...</p>

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-bold">Diagnoses ({total})</h2>
        <button
          onClick={handleTrigger}
          disabled={triggering}
          className="text-sm bg-blue-600 text-white px-3 py-1 rounded hover:bg-blue-700 disabled:opacity-50"
        >
          {triggering ? "Triggering..." : "Trigger"}
        </button>
      </div>
      {merged.length === 0 && <p className="text-gray-400 text-sm">No diagnoses yet.</p>}
      {merged.map((d) => (
        <div
          key={d.diagnosis_id}
          onClick={() => onSelect(d.diagnosis_id)}
          className="border rounded p-3 cursor-pointer hover:bg-gray-50 transition-colors"
        >
          <div className="flex items-center justify-between">
            <span className="font-mono text-xs text-gray-500">{d.diagnosis_id.slice(0, 8)}</span>
            <DiagnosisSeverityBadge severity={d.severity} />
          </div>
          <div className="text-sm mt-1">
            <span className="font-medium">{d.trigger_type}</span>
            <span className="text-gray-500 ml-2">{d.source_ids.join(", ")}</span>
          </div>
          {d.summary && <p className="text-xs text-gray-600 mt-1 line-clamp-2">{d.summary}</p>}
          {d.error_code && <p className="text-xs text-red-500 mt-1">{d.error_code}</p>}
        </div>
      ))}
      {items.length < total && (
        <button
          onClick={loadMore}
          className="w-full text-center text-sm text-blue-600 hover:underline py-2"
        >
          Load more ({total - offset - PAGE_SIZE > 0 ? total - offset - PAGE_SIZE : 0} remaining)
        </button>
      )}
    </div>
  )
}
