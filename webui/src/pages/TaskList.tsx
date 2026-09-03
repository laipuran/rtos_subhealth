import { useEffect, useState } from "react"
import { listTasks } from "../api/tasks"
import type { TaskRecord } from "../types/task"
import TaskStatusBadge from "../components/TaskStatusBadge"

const TYPE_EMOJI: Partial<Record<string, string>> = {
  go_to_tag: "📍",
  patrol_route: "🧭",
  hold: "⏸️",
}

function relTime(ts: number): string {
  const d = Math.floor((Date.now() / 1000 - ts) / 1000)
  if (d < 5) return "now"
  if (d < 60) return `${d}s`
  if (d < 3600) return `${Math.floor(d / 60)}m`
  if (d < 86400) return `${Math.floor(d / 3600)}h`
  return `${Math.floor(d / 86400)}d`
}

interface Props {
  refreshKey: number
  onSelect: (id: string) => void
  wsUpdates: Record<string, Partial<TaskRecord>>
}

const PAGE_SIZE = 20

export default function TaskList({ refreshKey, onSelect, wsUpdates }: Props) {
  const [tasks, setTasks] = useState<TaskRecord[]>([])
  const [total, setTotal] = useState(0)
  const [offset, setOffset] = useState(0)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    setOffset(0)
    setTasks([])
    listTasks(0, PAGE_SIZE)
      .then((data) => {
        setTasks(data.tasks)
        setTotal(data.total)
      })
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [refreshKey])

  const loadMore = async () => {
    const newOffset = offset + PAGE_SIZE
    const data = await listTasks(newOffset, PAGE_SIZE)
    setTasks((prev) => [...prev, ...data.tasks])
    setOffset(newOffset)
  }

  const merged = tasks.map((t) => {
    const upd = wsUpdates[t.goal_id]
    return upd ? { ...t, ...upd } : t
  })

  if (loading) return <p className="text-gray-400 text-sm">Loading...</p>

  return (
    <div className="space-y-2">
      <h2 className="text-lg font-bold">Tasks ({total})</h2>
      {merged.length === 0 && <p className="text-gray-400 text-sm">No tasks yet.</p>}
      {merged.map((t) => (
        <div
          key={t.goal_id}
          onClick={() => onSelect(t.goal_id)}
          className="border rounded p-3 cursor-pointer hover:bg-gray-50 transition-colors"
        >
          <div className="flex items-center justify-between">
            <span className="font-mono text-xs text-gray-500">{t.goal_id.slice(0, 8)}</span>
            <div className="flex items-center gap-2">
              <span className="text-xs text-gray-400">{relTime(t.created_at)}</span>
              <TaskStatusBadge state={t.state} />
            </div>
          </div>
          <div className="text-sm mt-1">
            <span className="font-medium">
              {TYPE_EMOJI[t.type] || ""} {t.type}
            </span>
            {t.target_tags?.length > 0 && (
              <span className="text-gray-500 ml-2">→ tag {t.target_tags.join(", ")}</span>
            )}
          </div>
          <div className="text-xs text-gray-400 mt-0.5">
            {t.current_tag !== -1 && `tag ${t.current_tag}`}
            {t.current_tag !== -1 && t.next_tag !== -1 && " → "}
            {t.next_tag !== -1 && `tag ${t.next_tag}`}
            {t.current_tag === -1 && t.next_tag === -1 && ""}
            {t.route?.length ? ` · ${t.route.length} 节点路径` : ""}
          </div>
          {t.error_code && <p className="text-xs text-red-500 mt-1">{t.error_code}</p>}
        </div>
      ))}
      {tasks.length < total && (
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
