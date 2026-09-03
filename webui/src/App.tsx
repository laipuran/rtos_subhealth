import { useCallback, useContext, createContext, useState, type ReactNode } from "react"
import {
  Routes,
  Route,
  Navigate,
  NavLink,
  useNavigate,
  useParams,
} from "react-router-dom"
import type { TaskRecord } from "./types/task"
import type { DiagnosisRecord } from "./types/diagnosis"
import { useTaskWS } from "./hooks/useTaskWS"
import { useDiagnosisWS } from "./hooks/useDiagnosisWS"
import { ToastProvider } from "./components/Toast"
import { VitalsProvider } from "./context/VitalsContext"
import TaskNew from "./pages/TaskNew"
import TaskList from "./pages/TaskList"
import TaskDetail from "./pages/TaskDetail"
import MapEditor from "./pages/MapEditor"
import DiagnosisList from "./pages/DiagnosisList"
import DiagnosisDetail from "./pages/DiagnosisDetail"
import VitalsChart from "./components/VitalsChart"

interface AppData {
  refreshKey: number
  bumpRefresh: () => void
  wsTasks: Record<string, Partial<TaskRecord>>
  wsDiags: Record<string, Partial<DiagnosisRecord>>
}

const AppDataContext = createContext<AppData | null>(null)

function useAppData(): AppData {
  const ctx = useContext(AppDataContext)
  if (!ctx) throw new Error("useAppData must be used within AppDataProvider")
  return ctx
}

function DataProvider({ children }: { children: ReactNode }) {
  const [refreshKey, setRefreshKey] = useState(0)
  const [wsTasks, setWsTasks] = useState<Record<string, Partial<TaskRecord>>>({})
  const [wsDiags, setWsDiags] = useState<Record<string, Partial<DiagnosisRecord>>>({})

  const handleTaskWs = useCallback((msg: any) => {
    setWsTasks((prev) => {
      const cur = prev[msg.goal_id] || {}
      if (msg.event === "feedback") {
        return { ...prev, [msg.goal_id]: { ...cur, state: msg.state, progress: msg.progress, current_tag: msg.current_tag, next_tag: msg.next_tag, error_code: msg.error_code, message: msg.message, route: msg.route, finished_stages: msg.finished_stages } }
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
        metrics: msg.metrics,
      },
    }))
  }, [])

  useTaskWS(handleTaskWs)
  useDiagnosisWS(handleDiagWs)

  return (
    <AppDataContext.Provider
      value={{
        refreshKey,
        bumpRefresh: () => setRefreshKey((k) => k + 1),
        wsTasks,
        wsDiags,
      }}
    >
      {children}
    </AppDataContext.Provider>
  )
}

const tabLinkClass = ({ isActive }: { isActive: boolean }) =>
  isActive ? "text-blue-600 font-medium" : "text-gray-500 hover:text-gray-700"

function TasksHome() {
  const { refreshKey, bumpRefresh, wsTasks } = useAppData()
  const navigate = useNavigate()
  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
      <div className="md:col-span-1">
        <TaskNew onCreated={bumpRefresh} />
      </div>
      <div className="md:col-span-2 space-y-4">
        <TaskList
          refreshKey={refreshKey}
          onSelect={(id) => navigate(`/tasks/${id}`)}
          wsUpdates={wsTasks}
        />
      </div>
    </div>
  )
}

function TaskDetailPage() {
  const { wsTasks } = useAppData()
  const { goalId } = useParams()
  const navigate = useNavigate()
  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
      <div className="md:col-span-1" />
      <div className="md:col-span-2 space-y-4">
        <TaskDetail
          goalId={goalId || null}
          onBack={() => navigate("/tasks")}
          liveUpdates={wsTasks}
        />
      </div>
    </div>
  )
}

function DiagnosesHome() {
  const { refreshKey, wsDiags } = useAppData()
  const navigate = useNavigate()
  return (
    <div className="space-y-3">
      <VitalsChart />
      <DiagnosisList
        refreshKey={refreshKey}
        onSelect={(id) => navigate(`/diagnoses/${id}`)}
        liveUpdates={wsDiags}
      />
    </div>
  )
}

function DiagnosesDetail() {
  const { wsDiags } = useAppData()
  const { id } = useParams()
  const navigate = useNavigate()
  return (
    <DiagnosisDetail
      diagnosisId={id || null}
      onBack={() => navigate("/diagnoses")}
      liveUpdates={wsDiags}
    />
  )
}

function MapPage() {
  const navigate = useNavigate()
  return (
    <div className="h-screen flex flex-col">
      <MapEditor onClose={() => navigate("/tasks")} />
    </div>
  )
}

function Layout() {
  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white border-b px-6 py-3 flex items-center justify-between">
        <h1 className="text-xl font-bold text-gray-800">Robot Task Console</h1>
        <div className="flex gap-4 items-center">
          <NavLink to="/tasks" className={tabLinkClass}>
            Tasks
          </NavLink>
          <NavLink to="/diagnoses" className={tabLinkClass}>
            Diagnoses
          </NavLink>
          <NavLink to="/map" className="text-sm text-blue-600 hover:underline">
            Edit Map
          </NavLink>
        </div>
      </header>

      <main className="max-w-4xl mx-auto p-6">
        <Routes>
          <Route path="/" element={<Navigate to="/tasks" replace />} />
          <Route path="/tasks" element={<TasksHome />} />
          <Route path="/tasks/:goalId" element={<TaskDetailPage />} />
          <Route path="/diagnoses" element={<DiagnosesHome />} />
          <Route path="/diagnoses/:id" element={<DiagnosesDetail />} />
          <Route path="/map" element={<MapPage />} />
          <Route path="*" element={<Navigate to="/tasks" replace />} />
        </Routes>
      </main>
    </div>
  )
}

export default function App() {
  return (
    <ToastProvider>
      <VitalsProvider>
        <DataProvider>
          <Layout />
        </DataProvider>
      </VitalsProvider>
    </ToastProvider>
  )
}
