from __future__ import annotations

import heapq
import json
import os
from typing import Optional

import rclpy
from rclpy.node import Node

from ros_interfaces.srv import PlanPath
from ros_interfaces.msg import Segment

_OK = "OK"
_PARTIAL = "PARTIAL"
_INVALID_GOAL = "INVALID_GOAL"
_GRAPH_MISSING = "GRAPH_MISSING"
_START_UNKNOWN = "START_UNKNOWN"
_TARGET_UNKNOWN = "TARGET_UNKNOWN"
_NO_ROUTE = "NO_ROUTE"


class PlannerNode(Node):
    def __init__(self) -> None:
        super().__init__("planner_node")

        maps_dir = self.declare_parameter("maps_dir", "").value
        if not maps_dir:
            import os as _os
            # Try ros2_ws/config/maps relative to the package install dir
            _path = _os.path.join(_os.path.dirname(__file__), "..", "..", "..", "..", "..", "config", "maps")
            maps_dir = _os.path.normpath(_path)
            if not _os.path.isdir(maps_dir):
                # Fallback to CWD
                maps_dir = _os.path.join(_os.getcwd(), "config", "maps")

        self._maps_dir = maps_dir
        self._graph: dict[int, list[tuple[int, float]]] = {}
        self._tags: dict[str, dict] = {}
        self._routes: dict[str, list[int]] = {}
        self._node_set: set[int] = set()
        self._load_map()

        self._srv = self.create_service(PlanPath, "plan_path", self._handle_plan)
        self.get_logger().info(
            f"Planner ready, {len(self._node_set)} nodes, "
            f"maps_dir={maps_dir}"
        )

    def _load_map(self) -> None:
        path = os.path.join(self._maps_dir, "default.json")
        if not os.path.exists(path):
            self.get_logger().error(f"Map file not found: {path}")
            return

        with open(path, "r") as f:
            data = json.load(f)

        self._tags = {str(k): v for k, v in data.get("tags", {}).items()}
        self._node_set = {int(k) for k in self._tags}
        self._routes = {str(k): v for k, v in data.get("routes", {}).items()}

        self._graph = {n: [] for n in self._node_set}
        for edge in data.get("edges", []):
            from_id = int(edge["from"])
            to_id = int(edge["to"])
            cost = float(edge["cost"])
            if from_id in self._graph and to_id in self._graph:
                self._graph[from_id].append((to_id, cost))

        self.get_logger().info(
            f"Loaded map: {len(self._node_set)} nodes, "
            f"{sum(len(v) for v in self._graph.values())} edges, "
            f"{len(self._routes)} routes"
        )

    def _handle_plan(self, request: PlanPath.Request, response: PlanPath.Response) -> PlanPath.Response:
        task_type = request.task_type
        target_tags = list(request.target_tags)
        start_tag = request.start_tag

        if not self._graph:
            response.error_code = _GRAPH_MISSING
            response.message = "graph not loaded"
            return response

        if not task_type:
            response.error_code = _INVALID_GOAL
            response.message = "empty task_type"
            return response

        if task_type == "hold":
            response.segments = []
            response.next_tag = -1
            response.error_code = _OK
            response.message = "hold: no movement needed"
            response.plan_id = ""
            return response

        if task_type == "patrol_route":
            if request.route_id and request.route_id in self._routes:
                target_tags = self._routes[request.route_id]
            if not target_tags:
                response.error_code = _INVALID_GOAL
                response.message = "patrol_route needs route_id or target_tags"
                return response
            segments = self._build_route_segments(target_tags)
            response.segments = segments
            response.next_tag = segments[0].to_tag if segments else -1
            response.error_code = _OK if segments else _NO_ROUTE
            response.message = f"patrol route: {len(segments)} segments"
            response.plan_id = ""
            return response

        if task_type == "go_to_tag":
            if not target_tags:
                response.error_code = _INVALID_GOAL
                response.message = "go_to_tag needs target_tags"
                return response

            target_id = target_tags[0]
            if target_id not in self._node_set:
                response.error_code = _TARGET_UNKNOWN
                response.message = f"target tag {target_id} not in graph"
                return response

            if start_tag != -1 and start_tag not in self._node_set:
                if request.allow_partial:
                    start_tag = min(self._node_set)
                else:
                    response.error_code = _START_UNKNOWN
                    response.message = f"start tag {start_tag} not in graph"
                    return response

            path = self._dijkstra(start_tag, target_id)
            if path is None:
                if request.allow_partial:
                    path = self._dijkstra(min(self._node_set), target_id)
                if path is None:
                    response.error_code = _NO_ROUTE
                    response.message = f"no path from {start_tag} to {target_id}"
                    response.segments = []
                    response.next_tag = -1
                    return response

            segments = self._path_to_segments(path)
            response.segments = segments
            response.next_tag = segments[0].to_tag if segments else -1
            response.error_code = _OK
            response.message = f"path found: {len(segments)} segments"
            response.plan_id = ""
            return response

        response.error_code = _INVALID_GOAL
        response.message = f"unknown task_type: {task_type}"
        return response

    def _dijkstra(self, start: int, goal: int) -> Optional[list[int]]:
        if start == goal:
            return [start]

        INF = float("inf")
        dist = {n: INF for n in self._node_set}
        prev = {n: None for n in self._node_set}
        dist[start] = 0.0

        pq = [(0.0, start)]
        while pq:
            d, u = heapq.heappop(pq)
            if d > dist[u]:
                continue
            if u == goal:
                break
            for v, cost in self._graph.get(u, []):
                nd = d + cost
                if nd < dist[v]:
                    dist[v] = nd
                    prev[v] = u
                    heapq.heappush(pq, (nd, v))

        if dist[goal] == INF:
            return None

        path = []
        cur = goal
        while cur is not None:
            path.append(cur)
            cur = prev[cur]
        path.reverse()
        return path

    def _path_to_segments(self, path: list[int]) -> list[Segment]:
        if len(path) < 2:
            return []
        segments = []
        for i in range(len(path) - 1):
            s = Segment()
            s.from_tag = path[i]
            s.to_tag = path[i + 1]
            s.edge_cost = self._lookup_edge_cost(path[i], path[i + 1])
            segments.append(s)
        return segments

    def _build_route_segments(self, tags: list[int]) -> list[Segment]:
        if len(tags) < 2:
            return []
        segments = []
        for i in range(len(tags) - 1):
            s = Segment()
            s.from_tag = tags[i]
            s.to_tag = tags[i + 1]
            s.edge_cost = self._lookup_edge_cost(tags[i], tags[i + 1])
            segments.append(s)
        return segments

    def _lookup_edge_cost(self, from_tag: int, to_tag: int) -> float:
        for efrom, elist in self._graph.items():
            for eto, ecost in elist:
                if efrom == from_tag and eto == to_tag:
                    return ecost
        # Estimate cost from Euclidean distance if tag coords available
        f = self._tags.get(str(from_tag))
        t = self._tags.get(str(to_tag))
        if f and t:
            dx = f["x"] - t["x"]
            dy = f["y"] - t["y"]
            return round((dx ** 2 + dy ** 2) ** 0.5, 1)
        return 0.0


def main() -> None:
    rclpy.init()
    node = PlannerNode()
    try:
        rclpy.spin(node)
    except KeyboardInterrupt:
        pass
    finally:
        node.destroy_node()
        rclpy.shutdown()


if __name__ == "__main__":
    main()
