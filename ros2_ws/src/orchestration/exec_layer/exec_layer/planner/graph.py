from __future__ import annotations

import heapq
import json
import os
from dataclasses import dataclass, field
from ros_interfaces.msg import Segment


@dataclass
class TagInfo:
    name: str
    x: float
    y: float


@dataclass
class Edge:
    from_tag: int
    to_tag: int
    cost: float
    edge_id: str = ""


@dataclass
class TagGraph:
    tags: dict[int, TagInfo] = field(default_factory=dict)
    edges: list[Edge] = field(default_factory=list)
    routes: dict[str, list[int]] = field(default_factory=dict)

    @classmethod
    def load_json(cls, filepath: str) -> "TagGraph":
        if not os.path.exists(filepath):
            raise FileNotFoundError(f"map file not found: {filepath}")

        with open(filepath, "r") as f:
            data = json.load(f)

        tags: dict[int, TagInfo] = {}
        for tag_id_str, info in data.get("tags", {}).items():
            tag_id = int(tag_id_str)
            tags[tag_id] = TagInfo(
                name=info.get("name", str(tag_id)),
                x=float(info.get("x", 0.0)),
                y=float(info.get("y", 0.0)),
            )

        edges: list[Edge] = []
        for i, e in enumerate(data.get("edges", [])):
            edges.append(Edge(
                from_tag=int(e["from"]),
                to_tag=int(e["to"]),
                cost=float(e.get("cost", 1.0)),
                edge_id=e.get("edge_id", f"{e['from']}->{e['to']}"),
            ))

        routes: dict[str, list[int]] = {}
        for route_id, route_tags in data.get("routes", {}).items():
            routes[route_id] = [int(t) for t in route_tags]

        return cls(tags=tags, edges=edges, routes=routes)

    def to_segments(self, path: list[int]) -> list[Segment]:
        segments: list[Segment] = []
        edge_map: dict[tuple[int, int], Edge] = {}
        for e in self.edges:
            edge_map[(e.from_tag, e.to_tag)] = e

        for i in range(len(path) - 1):
            f, t = path[i], path[i + 1]
            edge = edge_map.get((f, t))
            segments.append(Segment(
                from_tag=f,
                to_tag=t,
                edge_cost=edge.cost if edge else 1.0,
                edge_id=edge.edge_id if edge else f"{f}->{t}",
            ))
        return segments

    def dijkstra(
        self, start: int, target: int, avoid: set[int] | None = None,
    ) -> list[Segment]:
        if avoid is None:
            avoid = set()

        if start not in self.tags:
            return []
        if target not in self.tags:
            return []
        if start == target:
            return []

        adj: dict[int, list[tuple[int, float, str]]] = {}
        for tag_id in self.tags:
            adj[tag_id] = []
        for e in self.edges:
            if e.from_tag in avoid or e.to_tag in avoid:
                continue
            adj[e.from_tag].append((e.to_tag, e.cost, e.edge_id))

        dist: dict[int, float] = {start: 0.0}
        prev: dict[int, tuple[int, str]] = {}
        pq: list[tuple[float, int]] = [(0.0, start)]

        while pq:
            d, u = heapq.heappop(pq)
            if d > dist.get(u, float("inf")):
                continue
            if u == target:
                break
            for v, cost, edge_id in adj.get(u, []):
                nd = d + cost
                if nd < dist.get(v, float("inf")):
                    dist[v] = nd
                    prev[v] = (u, edge_id)
                    heapq.heappush(pq, (nd, v))

        if target not in prev and start != target:
            return []

        path: list[int] = []
        cur = target
        while cur != start:
            path.append(cur)
            if cur not in prev:
                return []
            cur = prev[cur][0]
        path.append(start)
        path.reverse()

        return self.to_segments(path)
