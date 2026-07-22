import { useEffect, useState } from "react"
import { getTask, cancelTask } from "../api/tasks"
import type { TaskRecord } from "../types/task"
import TaskStatusBadge from "../components/TaskStatusBadge"

interface Props {
  goalId: string | null
  onBack: () => void
  liveUpdates: Record<string, Partial<TaskRecord>>
}

export default function TaskDetail({ goalId, onBack, liveUpdates }: Props) {
  const [task, setTask] = useState<TaskRecord | null>(null)
  const [loading, setLoading] = useState(true)
  const [canceling, setCanceling] = useState(false)

  useEffect(() => {
    if (!goalId) return
    setLoading(true)
    getTask(goalId)
      .then(setTask)
      .catch(() => setTask(null))
      .finally(() => setLoading(false))
  }, [goalId])

  const merged = goalId && liveUpdates[goalId] ? { ...task, ...liveUpdates[goalId] } : task
  const rec = merged ? { ...merged, state: merged.state || "accepted" } : null

  const handleCancel = async () => {
    if (!goalId) return
    setCanceling(true)
    try {
      await cancelTask(goalId)
    } catch {
      /* ignore */
    }
    setCanceling(false)
  }

  if (!goalId) return null
  if (loading) return <p className="text-gray-400 text-sm">Loading...</p>
  if (!rec) return <p className="text-red-500 text-sm">Task not found.</p>

  const isActive = rec.state === "accepted" || rec.state === "running"

  return (
    <div className="space-y-4">
      <button onClick={onBack} className="text-sm text-blue-600 hover:underline">
        &larr; Back
      </button>

      <div className="flex items-center justify-between">
        <h2 className="text-lg font-bold">Task Detail</h2>
        <TaskStatusBadge state={rec.state} />
      </div>

      <table className="w-full text-sm">
        <tbody>
          {[
            ["ID", rec.goal_id],
            ["Type", rec.type],
            ["Target Tags", rec.target_tags?.join(", ") || "-"],
            ["Progress", rec.progress != null ? `${(rec.progress * 100).toFixed(0)}%` : "-"],
            ["Current Tag", rec.current_tag === -1 ? "-" : String(rec.current_tag)],
            ["Next Tag", rec.next_tag === -1 ? "-" : String(rec.next_tag)],
            ["Error Code", rec.error_code || "-"],
            ["Message", rec.message || "-"],
            ["Final State", rec.final_state || "-"],
          ].map(([label, val]) => (
            <tr key={label} className="border-b">
              <td className="py-1 pr-4 text-gray-500 font-medium">{label}</td>
              <td className="py-1">{val}</td>
            </tr>
          ))}
        </tbody>
      </table>

      {isActive && (
        <button
          onClick={handleCancel}
          disabled={canceling}
          className="bg-red-500 text-white px-4 py-2 rounded text-sm hover:bg-red-600 disabled:opacity-50"
        >
          {canceling ? "Canceling..." : "Cancel Task"}
        </button>
      )}
    </div>
  )
}
