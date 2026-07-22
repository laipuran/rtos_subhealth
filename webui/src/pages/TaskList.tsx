import { useEffect, useState } from "react"
import { listTasks } from "../api/tasks"
import type { TaskRecord } from "../types/task"
import TaskStatusBadge from "../components/TaskStatusBadge"

interface Props {
  refreshKey: number
  onSelect: (id: string) => void
  wsUpdates: Record<string, Partial<TaskRecord>>
}

export default function TaskList({ refreshKey, onSelect, wsUpdates }: Props) {
  const [tasks, setTasks] = useState<TaskRecord[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    listTasks()
      .then((data) => setTasks(data.tasks))
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [refreshKey])

  const merged = tasks.map((t) => {
    const upd = wsUpdates[t.goal_id]
    return upd ? { ...t, ...upd } : t
  })

  if (loading) return <p className="text-gray-400 text-sm">Loading...</p>

  if (merged.length === 0) return <p className="text-gray-400 text-sm">No tasks yet.</p>

  return (
    <div className="space-y-2">
      <h2 className="text-lg font-bold">Tasks</h2>
      {merged.map((t) => (
        <div
          key={t.goal_id}
          onClick={() => onSelect(t.goal_id)}
          className="border rounded p-3 cursor-pointer hover:bg-gray-50 transition-colors"
        >
          <div className="flex items-center justify-between">
            <span className="font-mono text-xs text-gray-500">{t.goal_id.slice(0, 8)}</span>
            <TaskStatusBadge state={t.state} />
          </div>
          <div className="text-sm mt-1">
            <span className="font-medium">{t.type}</span>
            {t.target_tags?.length > 0 && (
              <span className="text-gray-500 ml-2">tags: {t.target_tags.join(", ")}</span>
            )}
          </div>
          {t.error_code && <p className="text-xs text-red-500 mt-1">{t.error_code}</p>}
        </div>
      ))}
    </div>
  )
}
