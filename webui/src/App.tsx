import { useState, useCallback } from "react"
import type { TaskRecord } from "./types/task"
import type { DiagnosisRecord } from "./types/diagnosis"
import { useTaskWS } from "./hooks/useTaskWS"
import { useDiagnosisWS } from "./hooks/useDiagnosisWS"
import { ToastProvider } from "./components/Toast"
import TaskNew from "./pages/TaskNew"
import TaskList from "./pages/TaskList"
import TaskDetail from "./pages/TaskDetail"
import MapEditor from "./pages/MapEditor"
import DiagnosisList from "./pages/DiagnosisList"
import DiagnosisDetail from "./pages/DiagnosisDetail"

type Tab = "tasks" | "diagnoses"

function Main() {
  const [tab, setTab] = useState<Tab>("tasks")
  const [refreshKey, setRefreshKey] = useState(0)
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null)
  const [selectedDiagId, setSelectedDiagId] = useState<string | null>(null)
  const [wsTasks, setWsTasks] = useState<Record<string, Partial<TaskRecord>>>({})
  const [wsDiags, setWsDiags] = useState<Record<string, Partial<DiagnosisRecord>>>({})
  const [showEditor, setShowEditor] = useState(false)

  const handleTaskWs = useCallback((msg: any) => {
    setWsTasks((prev) => {
      const cur = prev[msg.goal_id] || {}
      if (msg.event === "feedback") {
        return { ...prev, [msg.goal_id]: { ...cur, state: msg.state, progress: msg.progress, current_tag: msg.current_tag, next_tag: msg.next_tag, error_code: msg.error_code, message: msg.message } }
      }
      if (msg.event === "result") {
        return { ...prev, [msg.goal_id]: { ...cur, state: msg.final_state, final_state: msg.final_state, error_code: msg.error_code, message: msg.message } }
      }
      return prev
    })
  }, [])

  const handleDiagWs = useCallback((msg: any) => {
    setWsDiags((prev) => ({
      ...prev,
      [msg.diagnosis_id]: {
        severity: msg.severity,
        summary: msg.summary,
        possible_causes: msg.possible_causes,
        recommendations: msg.recommendations,
        confidence: msg.confidence,
        error_code: msg.error_code,
        error_message: msg.error_message,
      },
    }))
  }, [])

  useTaskWS(handleTaskWs)
  useDiagnosisWS(handleDiagWs)

  if (showEditor) {
    return (
      <div className="h-screen flex flex-col">
        <MapEditor onClose={() => setShowEditor(false)} />
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white border-b px-6 py-3 flex items-center justify-between">
        <h1 className="text-xl font-bold text-gray-800">Robot Task Console</h1>
        <div className="flex gap-4">
          <button
            onClick={() => setTab("tasks")}
            className={tab === "tasks" ? "text-blue-600 font-medium" : "text-gray-500 hover:text-gray-700"}
          >
            Tasks
          </button>
          <button
            onClick={() => setTab("diagnoses")}
            className={tab === "diagnoses" ? "text-blue-600 font-medium" : "text-gray-500 hover:text-gray-700"}
          >
            Diagnoses
          </button>
          <button onClick={() => setShowEditor(true)} className="text-sm text-blue-600 hover:underline">
            Edit Map
          </button>
        </div>
      </header>

      <main className="max-w-4xl mx-auto p-6 grid grid-cols-1 md:grid-cols-3 gap-6">
        {tab === "tasks" ? (
          <>
            <div className="md:col-span-1">
              <TaskNew onCreated={() => setRefreshKey((k) => k + 1)} />
            </div>
            <div className="md:col-span-2 space-y-4">
              {selectedTaskId ? (
                <TaskDetail goalId={selectedTaskId} onBack={() => setSelectedTaskId(null)} liveUpdates={wsTasks} />
              ) : (
                <TaskList refreshKey={refreshKey} onSelect={setSelectedTaskId} wsUpdates={wsTasks} />
              )}
            </div>
          </>
        ) : (
          <>
            <div className="md:col-span-3 space-y-4">
              {selectedDiagId ? (
                <DiagnosisDetail diagnosisId={selectedDiagId} onBack={() => setSelectedDiagId(null)} liveUpdates={wsDiags} />
              ) : (
                <DiagnosisList refreshKey={refreshKey} onSelect={setSelectedDiagId} liveUpdates={wsDiags} />
              )}
            </div>
          </>
        )}
      </main>
    </div>
  )
}

export default function App() {
  return (
    <ToastProvider>
      <Main />
    </ToastProvider>
  )
}
