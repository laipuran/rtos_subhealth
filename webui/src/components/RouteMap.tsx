import { useEffect, useMemo, useState } from "react"
import {
  ReactFlow,
  Background,
  Controls,
  type Node,
  type Edge,
  MarkerType,
} from "@xyflow/react"
import "@xyflow/react/dist/style.css"
import { getMap } from "../api/map"
import type { TagGraph, TagNode } from "../types/map"

interface Props {
  route?: number[]
  currentTag?: number
  targetTags?: number[]
  finishedStages?: number
}

const NODE_W = 64
const NODE_H = 34
const ROUTE_COLOR = "#2563eb"

function buildNodes(graph: TagGraph): Node[] {
  return Object.entries(graph.tags).map(([id, tag]) => ({
    id,
    position: { x: (tag.x ?? 0) * 100, y: (tag.y ?? 0) * 100 },
    data: { label: `${id}·${tag.name}` },
    style: {
      width: NODE_W,
      height: NODE_H,
      background: "#fff",
      border: "1px solid #cbd5e1",
      borderRadius: 8,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      fontSize: 11,
      padding: 0,
    },
  }))
}

function buildBaseEdges(graph: TagGraph): Edge[] {
  return graph.edges.map((e, i) => ({
    id: `b${i}`,
    source: String(e.from),
    target: String(e.to),
    label: String(e.cost),
    type: "smoothstep",
    style: { stroke: "#e2e8f0", strokeWidth: 1 },
    markerEnd: { type: MarkerType.ArrowClosed, color: "#e2e8f0" },
  }))
}

export default function RouteMap({ route, currentTag, targetTags, finishedStages }: Props) {
  const [graph, setGraph] = useState<TagGraph | null>(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    getMap()
      .then(setGraph)
      .catch(() => setError(true))
  }, [])

  const { nodes, edges } = useMemo(() => {
    if (!graph) return { nodes: [] as Node[], edges: [] as Edge[] }
    const nodeList = buildNodes(graph)
    const base = buildBaseEdges(graph)

    if (route && route.length > 0) {
      const valid = route.filter((t) => graph.tags[String(t)] != null)
      const routeEdges: Edge[] = []
      for (let i = 0; i < valid.length - 1; i++) {
        routeEdges.push({
          id: `r${i}`,
          source: String(valid[i]),
          target: String(valid[i + 1]),
          type: "smoothstep",
          animated: true,
          style: { stroke: ROUTE_COLOR, strokeWidth: 3 },
          markerEnd: { type: MarkerType.ArrowClosed, color: ROUTE_COLOR },
        })
      }

      const target = targetTags && targetTags.length > 0 ? targetTags[0] : valid[valid.length - 1]
      const targetId = String(target)
      for (const n of nodeList) {
        let style = { ...n.style }
        if (n.id === targetId) {
          style.border = "2px solid #16a34a"
        } else if (n.id === String(currentTag)) {
          style.border = "2px solid #f59e0b"
        }
        n.style = style
      }
      return { nodes: nodeList, edges: [...routeEdges, ...base] }
    }

    return { nodes: nodeList, edges: base }
  }, [graph, route, currentTag, targetTags])

  if (error) {
    return <p className="text-xs text-red-500">加载地图失败</p>
  }
  if (!graph) {
    return <div className="h-52 flex items-center justify-center text-sm text-gray-400">加载地图…</div>
  }

  return (
    <div className="border rounded bg-white overflow-hidden">
      <div className="flex items-center justify-between px-3 py-1.5 text-xs text-gray-500 border-b bg-gray-50">
        <span>路径示意</span>
        {finishedStages != null && route && route.length > 0 && (
          <span>
            已完成 {finishedStages} / {route.length - 1} 段
          </span>
        )}
      </div>
      <div style={{ height: 240 }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          fitView
          nodesDraggable={false}
          nodesConnectable={false}
          elementsSelectable={false}
          proOptions={{ hideAttribution: true }}
        >
          <Background gap={16} />
          <Controls showInteractive={false} />
        </ReactFlow>
      </div>
    </div>
  )
}
