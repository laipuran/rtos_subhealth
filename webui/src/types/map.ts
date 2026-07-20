export interface TagNode {
  name: string
  x: number
  y: number
}

export interface TagEdge {
  from: number
  to: number
  cost: number
}

export interface TagGraph {
  tags: Record<string, TagNode>
  edges: TagEdge[]
  routes: Record<string, number[]>
}
