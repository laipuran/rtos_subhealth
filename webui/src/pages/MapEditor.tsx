import { useCallback, useEffect, useState } from "react"
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  addEdge,
  type Connection,
  type Node,
  type Edge,
  MarkerType,
} from "@xyflow/react"
import "@xyflow/react/dist/style.css"

import { getMap, saveMap } from "../api/map"
import { listTasks } from "../api/tasks"
import type { TagGraph, TagNode as TagNodeData } from "../types/map"
import { useToast } from "../components/Toast"

const NODE_WIDTH = 80
const NODE_HEIGHT = 40

function buildNodes(
  tags: Record<string, TagNodeData>,
  blockedTagIds: Set<string>,
): Node[] {
  return Object.entries(tags).map(([id, tag]) => ({
    id,
    type: "default",
    position: { x: tag.x * 100, y: tag.y * 100 },
    data: { label: `${id}: ${tag.name}`, disabled: blockedTagIds.has(id) },
    style: {
      width: NODE_WIDTH,
      height: NODE_HEIGHT,
      background: blockedTagIds.has(id) ? "#d1d5db" : "#e0f2fe",
      border: blockedTagIds.has(id) ? "1px solid #9ca3af" : "1px solid #38bdf8",
      borderRadius: 8,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      fontSize: 12,
      cursor: blockedTagIds.has(id) ? "not-allowed" : "grab",
      opacity: blockedTagIds.has(id) ? 0.6 : 1,
    },
  }))
}

function buildEdges(edgesList: TagGraph["edges"]): Edge[] {
  return edgesList.map((e, i) => ({
    id: `e${i}`,
    source: String(e.from),
    target: String(e.to),
    label: String(e.cost),
    type: "smoothstep",
    markerEnd: { type: MarkerType.ArrowClosed },
    style: { stroke: "#64748b" },
  }))
}

function graphFromFlow(nodes: Node[], edges: Edge[]): TagGraph {
  const tags: Record<string, TagNodeData> = {}
  for (const n of nodes) {
    const label = n.data.label as string
    const tagId = n.id
    const name = label.includes(":") ? label.split(": ")[1] : label
    tags[tagId] = {
      name,
      x: Math.round((n.position.x / 100) * 10) / 10,
      y: Math.round((n.position.y / 100) * 10) / 10,
    }
  }
  const edgeList = edges.map((e) => ({
    from: Number(e.source),
    to: Number(e.target),
    cost: Number(e.label) || 1,
  }))
  return { tags, edges: edgeList, routes: {} }
}

interface Props {
  onClose: () => void
}

export default function MapEditor({ onClose }: Props) {
  const { toast } = useToast()
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([])
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([])
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState("")
  const [blockedTags, setBlockedTags] = useState<Set<string>>(new Set())
  const [nextId, setNextId] = useState(100)
  const [loaded, setLoaded] = useState(false)

  useEffect(() => {
    Promise.all([getMap(), listTasks()])
      .then(([graph, tasks]) => {
        const activeTagIds = new Set<string>()
        for (const t of tasks.tasks) {
          if (t.state === "accepted" || t.state === "running") {
            for (const tag of t.target_tags || []) {
              activeTagIds.add(String(tag))
            }
          }
        }
        setBlockedTags(activeTagIds)
        setNodes(buildNodes(graph.tags, activeTagIds))
        setEdges(buildEdges(graph.edges))
        const maxId = Math.max(0, ...Object.keys(graph.tags).map(Number))
        setNextId(maxId + 1)
      })
      .catch(() => setError("failed to load map"))
      .finally(() => setLoaded(true))
  }, [setNodes, setEdges])

  const onConnect = useCallback(
    (conn: Connection) => {
      setEdges((eds) => {
        const newEdge: Edge = {
          ...conn,
          id: `e${Date.now()}`,
          label: "1.0",
          type: "smoothstep",
          markerEnd: { type: MarkerType.ArrowClosed },
          style: { stroke: "#64748b" },
        }
        return addEdge(newEdge, eds)
      })
    },
    [setEdges],
  )

  const onAddNode = useCallback(() => {
    const id = String(nextId)
    setNextId((n) => n + 1)
    const newNode: Node = {
      id,
      type: "default",
      position: { x: 200 + Math.random() * 200, y: 200 + Math.random() * 200 },
      data: { label: `${id}: new_tag`, disabled: false },
      style: {
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
        background: "#e0f2fe",
        border: "1px solid #38bdf8",
        borderRadius: 8,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontSize: 12,
      },
    }
    setNodes((nds) => [...nds, newNode])
  }, [nextId, setNodes])

  const onDeleteSelected = useCallback(
    (nodesToDelete: Node[]) => {
      for (const n of nodesToDelete) {
        if (blockedTags.has(n.id)) {
          setError(`tag ${n.id} is blocked by active task, cannot delete`)
          return
        }
      }
      setError("")
    },
    [blockedTags],
  )

  const handleSave = async () => {
    setSaving(true)
    setError("")
    try {
      const graph = graphFromFlow(nodes, edges)
      await saveMap(graph)
      toast("Map saved!", "success")
    } catch (err: any) {
      setError(err.message || "save failed")
    }
    setSaving(false)
  }

  if (!loaded) return <p className="p-4 text-gray-400">Loading map...</p>

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between px-4 py-2 bg-white border-b">
        <h2 className="text-lg font-bold">Tag Graph Editor</h2>
        <div className="flex gap-2">
          <button
            onClick={onAddNode}
            className="px-3 py-1 bg-blue-500 text-white rounded text-sm hover:bg-blue-600"
          >
            + Add Tag
          </button>
          <button
            onClick={onClose}
            className="px-3 py-1 bg-gray-200 rounded text-sm hover:bg-gray-300"
          >
            Back
          </button>
          <button
            onClick={handleSave}
            disabled={saving}
            className="px-3 py-1 bg-green-600 text-white rounded text-sm hover:bg-green-700 disabled:opacity-50"
          >
            {saving ? "Saving..." : "Save"}
          </button>
        </div>
      </div>

      {error && (
        <div className="px-4 py-2 bg-red-50 text-red-600 text-sm border-b">{error}</div>
      )}

      <div className="flex-1">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onNodesDelete={onDeleteSelected}
          fitView
        >
          <Background />
          <Controls />
          <MiniMap />
        </ReactFlow>
      </div>
    </div>
  )
}
