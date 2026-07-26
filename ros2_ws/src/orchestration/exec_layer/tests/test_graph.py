import sys, json, os, types, importlib.util

# Mock ros_interfaces.msg.Segment so tests run without ROS2
_segment_mod = types.ModuleType("ros_interfaces.msg")


class _MockSegment:
    def __init__(self, from_tag=0, to_tag=0, edge_cost=1.0, edge_id=""):
        self.from_tag = from_tag
        self.to_tag = to_tag
        self.edge_cost = edge_cost
        self.edge_id = edge_id

    def __repr__(self):
        return f"Segment({self.from_tag}->{self.to_tag}, cost={self.edge_cost})"


_segment_mod.Segment = _MockSegment
sys.modules["ros_interfaces"] = types.ModuleType("ros_interfaces")
sys.modules["ros_interfaces.msg"] = _segment_mod

# Load graph module directly (skip planner.__init__ which imports rclpy)
_graph_path = os.path.join(
    os.path.dirname(__file__), "..", "exec_layer", "planner", "graph.py"
)
spec = importlib.util.spec_from_file_location(
    "exec_layer.planner.graph", _graph_path, submodule_search_locations=[]
)
_graph_mod = importlib.util.module_from_spec(spec)
sys.modules["exec_layer.planner.graph"] = _graph_mod
spec.loader.exec_module(_graph_mod)

TagGraph = _graph_mod.TagGraph
TagInfo = _graph_mod.TagInfo
Edge = _graph_mod.Edge

MAP_PATH = "ros2_ws/config/maps/default.json"


class TestTagGraphLoading:
    def test_load_valid_json(self):
        g = TagGraph.load_json(MAP_PATH)
        assert len(g.tags) == 4
        assert len(g.edges) == 4
        assert len(g.routes) == 2

    def test_tag_contents(self):
        g = TagGraph.load_json(MAP_PATH)
        assert g.tags[1].name == "bed_1"
        assert g.tags[1].x == 0.0
        assert g.tags[1].y == 0.0
        assert g.tags[42].name == "target"

    def test_edge_contents(self):
        g = TagGraph.load_json(MAP_PATH)
        edge = g.edges[0]
        assert edge.from_tag == 1
        assert edge.to_tag == 2
        assert edge.cost == 3.0

    def test_routes(self):
        g = TagGraph.load_json(MAP_PATH)
        assert "patrol_ward" in g.routes
        assert "supply_point" in g.routes
        assert g.routes["supply_point"] == [1, 42]

    def test_load_nonexistent_file(self):
        import pytest
        with pytest.raises(FileNotFoundError):
            TagGraph.load_json("nonexistent.json")


class TestDijkstra:
    def test_shortest_path(self):
        g = TagGraph.load_json(MAP_PATH)
        segs = g.dijkstra(1, 42)
        assert len(segs) == 3
        assert segs[0].from_tag == 1
        assert segs[0].to_tag == 2
        assert segs[1].to_tag == 3
        assert segs[2].to_tag == 42
        total_cost = sum(s.edge_cost for s in segs)
        assert total_cost == 9.5

    def test_path_with_avoid(self):
        g = TagGraph.load_json(MAP_PATH)
        segs = g.dijkstra(1, 42, avoid={2})
        assert len(segs) == 1
        assert segs[0].from_tag == 1
        assert segs[0].to_tag == 42
        assert segs[0].edge_cost == 10.0

    def test_target_not_in_graph(self):
        g = TagGraph.load_json(MAP_PATH)
        segs = g.dijkstra(1, 999)
        assert segs == []

    def test_start_not_in_graph(self):
        g = TagGraph.load_json(MAP_PATH)
        segs = g.dijkstra(99, 1)
        assert segs == []

    def test_start_equals_target(self):
        g = TagGraph.load_json(MAP_PATH)
        segs = g.dijkstra(1, 1)
        assert segs == []

    def test_no_route(self):
        g = TagGraph.load_json(MAP_PATH)
        segs = g.dijkstra(2, 1)
        assert segs == []

    def test_avoid_makes_unreachable(self):
        g = TagGraph.load_json(MAP_PATH)
        # avoid all neighbors of 1 -> no path possible
        segs = g.dijkstra(1, 42, avoid={1, 2, 3, 42})
        assert segs == []


class TestToSegments:
    def test_to_segments_creates_correct_edges(self):
        g = TagGraph.load_json(MAP_PATH)
        segs = g.to_segments([1, 2, 3])
        assert len(segs) == 2
        assert segs[0].from_tag == 1
        assert segs[0].to_tag == 2
        assert segs[1].from_tag == 2
        assert segs[1].to_tag == 3


class TestEdgeCases:
    def test_empty_graph(self):
        g = TagGraph(tags={}, edges=[], routes={})
        segs = g.dijkstra(1, 2)
        assert segs == []

    def test_single_node_graph(self):
        g = TagGraph(tags={1: TagInfo(name="a", x=0, y=0)}, edges=[], routes={})
        segs = g.dijkstra(1, 1)
        assert segs == []

    def test_direct_edge_only(self):
        g = TagGraph(
            tags={1: TagInfo("a", 0, 0), 2: TagInfo("b", 1, 1)},
            edges=[Edge(1, 2, 5.0, "e1")],
            routes={},
        )
        segs = g.dijkstra(1, 2)
        assert len(segs) == 1
        assert segs[0].edge_id == "e1"
        assert segs[0].edge_cost == 5.0
