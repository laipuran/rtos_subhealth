import { useState, useCallback } from "react"
import type { TaskRecord } from "./types/task"
import { useTaskWS } from "./hooks/useTaskWS"
import TaskNew from "./pages/TaskNew"
import TaskList from "./pages/TaskList"
import TaskDetail from "./pages/TaskDetail"

export default function App() {
  const [refreshKey, setRefreshKey] = useState(0)
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [wsUpdates, setWsUpdates] = useState<Record<string, Partial<TaskRecord>>>({})

  const handleWsMsg = useCallback((msg: any) => {
    setWsUpdates((prev) => {
      const cur = prev[msg.goal_id] || {}
      if (msg.event === "feedback") {
        return {
          ...prev,
          [msg.goal_id]: { ...cur, state: msg.state, progress: msg.progress, current_tag: msg.current_tag, next_tag: msg.next_tag, error_code: msg.error_code, message: msg.message },
        }
      }
      if (msg.event === "result") {
        return {
          ...prev,
          [msg.goal_id]: { ...cur, state: msg.final_state, final_state: msg.final_state, error_code: msg.error_code, message: msg.message },
        }
      }
      return prev
    })
  }, [])

  useTaskWS(handleWsMsg)

  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white border-b px-6 py-3">
        <h1 className="text-xl font-bold text-gray-800">Robot Task Console</h1>
      </header>

      <main className="max-w-4xl mx-auto p-6 grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="md:col-span-1">
          <TaskNew onCreated={() => setRefreshKey((k) => k + 1)} />
        </div>

        <div className="md:col-span-2 space-y-4">
          {selectedId ? (
            <TaskDetail
              goalId={selectedId}
              onBack={() => setSelectedId(null)}
              liveUpdates={wsUpdates}
            />
          ) : (
            <TaskList
              refreshKey={refreshKey}
              onSelect={setSelectedId}
              wsUpdates={wsUpdates}
            />
          )}
        </div>
      </main>
    </div>
  )
}
