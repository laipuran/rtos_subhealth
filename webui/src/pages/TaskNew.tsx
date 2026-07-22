import { useState } from "react"
import { createTask } from "../api/tasks"

interface Props {
  onCreated: () => void
}

export default function TaskNew({ onCreated }: Props) {
  const [type, setType] = useState<"go_to_tag" | "patrol_route" | "hold">("go_to_tag")
  const [tags, setTags] = useState("")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState("")

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError("")

    try {
      const targetTags = tags
        .split(/[,\s]+/)
        .map((s) => Number(s.trim()))
        .filter((n) => !isNaN(n))

      await createTask({
        type,
        target_tags: targetTags.length > 0 ? targetTags : undefined,
      })
      setTags("")
      onCreated()
    } catch (err: any) {
      setError(err.message || "create task failed")
    } finally {
      setLoading(false)
    }
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <h2 className="text-lg font-bold">New Task</h2>

      <div>
        <label className="block text-sm font-medium mb-1">Type</label>
        <select
          value={type}
          onChange={(e) => setType(e.target.value as any)}
          className="w-full border rounded px-3 py-2 text-sm"
        >
          <option value="go_to_tag">Go to Tag</option>
          <option value="patrol_route">Patrol Route</option>
          <option value="hold">Hold</option>
        </select>
      </div>

      {type !== "hold" && (
        <div>
          <label className="block text-sm font-medium mb-1">
            Target Tags <span className="text-gray-400">(comma or space separated)</span>
          </label>
          <input
            type="text"
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            placeholder="e.g. 42, 43, 44"
            className="w-full border rounded px-3 py-2 text-sm"
          />
        </div>
      )}

      {error && <p className="text-red-600 text-sm">{error}</p>}

      <button
        type="submit"
        disabled={loading}
        className="bg-blue-600 text-white px-4 py-2 rounded text-sm font-medium hover:bg-blue-700 disabled:opacity-50"
      >
        {loading ? "Submitting..." : "Submit Task"}
      </button>
    </form>
  )
}
