from __future__ import annotations

import os
from typing import Optional

import rclpy
from rclpy.node import Node

from ros_interfaces.srv import PlanPath

from .graph import TagGraph


class PlannerNode(Node):
    def __init__(self) -> None:
        super().__init__("planner_node")

        maps_dir = self.declare_parameter("maps_dir", "").value
        map_scene = self.declare_parameter("map_scene", "default").value

        self._graph: Optional[TagGraph] = None
        self._maps_dir = maps_dir

        if maps_dir:
            self._load_graph(maps_dir, map_scene)
        else:
            self.get_logger().warn("maps_dir not set, planner will reject all requests")

        self._srv = self.create_service(PlanPath, "plan_path", self._handle_plan)
        self.get_logger().info(
            f"Planner ready, maps_dir={maps_dir}, scene={map_scene}"
        )

    def _load_graph(self, maps_dir: str, scene: str) -> None:
        filepath = os.path.join(maps_dir, f"{scene}.json")
        try:
            self._graph = TagGraph.load_json(filepath)
            self.get_logger().info(
                f"Loaded graph: {len(self._graph.tags)} tags, "
                f"{len(self._graph.edges)} edges, "
                f"{len(self._graph.routes)} routes"
            )
        except FileNotFoundError:
            self.get_logger().error(f"Map file not found: {filepath}")
        except Exception as e:
            self.get_logger().error(f"Failed to load map: {e}")

    def reload(self) -> None:
        if self._maps_dir:
            scene = self.get_parameter("map_scene").value
            self._load_graph(self._maps_dir, scene)

    def _handle_plan(self, request: PlanPath.Request, response: PlanPath.Response) -> PlanPath.Response:
        if self._graph is None:
            response.error_code = "GRAPH_MISSING"
            response.message = "planner has no graph loaded"
            response.plan_id = ""
            response.next_tag = -1
            response.segments = []
            return response

        avoid = set(request.constraints.avoid_tags) if request.constraints else set()

        if request.task_type == "hold":
            response.error_code = "OK"
            response.message = "hold: no movement needed"
            response.plan_id = request.goal_id or ""
            response.next_tag = -1
            response.segments = []
            return response

        if request.task_type == "go_to_tag":
            return self._plan_go_to_tag(request, response, avoid)

        if request.task_type == "patrol_route":
            return self._plan_patrol_route(request, response, avoid)

        response.error_code = "INVALID_GOAL"
        response.message = f"unknown task_type: {request.task_type}"
        response.plan_id = ""
        response.next_tag = -1
        response.segments = []
        return response

    def _plan_go_to_tag(
        self, request: PlanPath.Request, response: PlanPath.Response,
        avoid: set[int],
    ) -> PlanPath.Response:
        if not request.target_tags:
            response.error_code = "INVALID_GOAL"
            response.message = "go_to_tag requires exactly one target_tag"
            response.segments = []
            response.next_tag = -1
            response.plan_id = request.goal_id or ""
            return response

        target = request.target_tags[0]
        if target not in self._graph.tags:
            response.error_code = "TARGET_UNKNOWN"
            response.message = f"target tag {target} not in graph"
            response.segments = []
            response.next_tag = -1
            response.plan_id = request.goal_id or ""
            return response

        start_tag = request.start_tag
        if start_tag == -1 or start_tag not in self._graph.tags:
            response.error_code = "START_UNKNOWN"
            response.message = (
                f"start tag {start_tag} unknown, "
                "exec_layer must infer from detections"
            )
            response.segments = []
            response.next_tag = -1
            response.plan_id = request.goal_id or ""
            return response

        if start_tag == target:
            response.error_code = "OK"
            response.message = "already at target tag"
            response.segments = []
            response.next_tag = -1
            response.plan_id = request.goal_id or ""
            return response

        segments = self._graph.dijkstra(start_tag, target, avoid)
        if not segments:
            response.error_code = "NO_ROUTE"
            response.message = f"no path from {start_tag} to {target}"
            response.segments = []
            response.next_tag = -1
            response.plan_id = request.goal_id or ""
            return response

        response.error_code = "OK"
        response.message = ""
        response.segments = segments
        response.next_tag = segments[0].to_tag
        response.plan_id = request.goal_id or ""
        return response

    def _plan_patrol_route(
        self, request: PlanPath.Request, response: PlanPath.Response,
        avoid: set[int],
    ) -> PlanPath.Response:
        route_id = request.route_id
        if not route_id or route_id not in self._graph.routes:
            response.error_code = "INVALID_GOAL"
            response.message = f"route_id '{route_id}' not found"
            response.segments = []
            response.next_tag = -1
            response.plan_id = request.goal_id or ""
            return response

        route_tags = self._graph.routes[route_id]
        all_segments: list = []
        for i in range(len(route_tags) - 1):
            f, t = route_tags[i], route_tags[i + 1]
            segs = self._graph.dijkstra(f, t, avoid)
            if not segs:
                response.error_code = "NO_ROUTE"
                response.message = f"no path from {f} to {t} in route {route_id}"
                response.segments = []
                response.next_tag = -1
                response.plan_id = request.goal_id or ""
                return response
            all_segments.extend(segs)

        response.error_code = "OK"
        response.message = ""
        response.segments = all_segments
        response.next_tag = all_segments[0].to_tag if all_segments else -1
        response.plan_id = request.goal_id or ""
        return response


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
