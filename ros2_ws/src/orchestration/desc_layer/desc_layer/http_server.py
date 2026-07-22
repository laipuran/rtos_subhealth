from __future__ import annotations

import json
import os
import threading
from typing import Any, Optional

import flask
from flask import Flask, Response, request
from flask_sock import Sock

from ros_interfaces.action import ExecTask

from .task_store import TaskRecord, TaskStore

app = Flask(__name__)
sock = Sock(app)

task_store: TaskStore = None
goal_queue: "queue.Queue[Optional[dict]]" = None
maps_path: str = ""
ws_clients: list = []
_ws_lock = threading.Lock()


def init_app(store: TaskStore, queue: "queue.Queue[Optional[dict]]",
             maps_dir: str = "") -> None:
    global task_store, goal_queue, maps_path
    task_store = store
    goal_queue = queue
    maps_path = maps_dir


def _json_resp(data: Any, status: int = 200) -> Response:
    return flask.Response(
        json.dumps(data, ensure_ascii=False),
        status=status,
        content_type="application/json",
    )


def _load_map(scene: str = "default") -> Optional[dict]:
    filepath = os.path.join(maps_path, f"{scene}.json")
    if not os.path.exists(filepath):
        return None
    with open(filepath, "r") as f:
        return json.load(f)


def _save_map(data: dict, scene: str = "default") -> None:
    filepath = os.path.join(maps_path, f"{scene}.json")
    with open(filepath, "w") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)


def _validate_goal(body: dict) -> Optional[tuple[str, int]]:
    goal = body.get("goal")
    if not goal:
        return "missing goal field", 400
    if not goal.get("type"):
        return "missing goal.type", 400
    valid_types = {"go_to_tag", "patrol_route", "hold"}
    if goal["type"] not in valid_types:
        return f"invalid type, must be one of {valid_types}", 400
    return None


def _build_goal_from_body(body: dict) -> ExecTask.Goal:
    goal = body["goal"]
    g = ExecTask.Goal()
    g.type = goal.get("type", "")
    g.priority = goal.get("priority", 0)
    g.route_id = goal.get("route_id", "")
    g.target_tags = goal.get("target_tags", [])
    constraints = goal.get("constraints", {})
    g.constraints.max_speed_mps = constraints.get("max_speed_mps", 0.0)
    g.constraints.min_clearance_m = constraints.get("min_clearance_m", 0.0)
    g.constraints.avoid_tags = constraints.get("avoid_tags", [])
    g.deadline_ms = goal.get("deadline_ms", 0)
    return g


def _check_conflicts(old: dict, new: dict) -> Optional[dict]:
    old_tag_ids = set(old.get("tags", {}).keys())
    new_tag_ids = set(new.get("tags", {}).keys())
    deleted_tags = old_tag_ids - new_tag_ids

    old_edge_keys = {(e["from"], e["to"]) for e in old.get("edges", [])}
    new_edge_keys = {(e["from"], e["to"]) for e in new.get("edges", [])}
    deleted_edges = old_edge_keys - new_edge_keys

    old_route_keys = set(old.get("routes", {}).keys())
    new_route_keys = set(new.get("routes", {}).keys())
    deleted_routes = old_route_keys - new_route_keys

    if not deleted_tags and not deleted_edges and not deleted_routes:
        return None

    active_tasks = task_store.list_active()
    blocking = []
    for t in active_tasks:
        reasons = []
        for tag_id in t.goal.target_tags:
            if str(tag_id) in deleted_tags:
                reasons.append(f"target_tag {tag_id}")
        if t.goal.route_id in deleted_routes:
            reasons.append(f"route_id {t.goal.route_id}")
        if reasons:
            blocking.append({
                "goal_id": t.goal_id,
                "type": t.goal.type,
                "target_tags": list(t.goal.target_tags),
                "route_id": t.goal.route_id,
                "state": t.state,
                "reasons": reasons,
            })

    if not blocking:
        return None

    return {
        "error": "cannot delete items used by active task(s)",
        "deleted_tags": sorted(deleted_tags),
        "deleted_edges": [{"from": e[0], "to": e[1]} for e in deleted_edges],
        "deleted_routes": sorted(deleted_routes),
        "blocking_tasks": blocking,
    }


def broadcast(goal_id: str, event_type: str, data: dict) -> None:
    payload = json.dumps({"goal_id": goal_id, "event": event_type, **data}, ensure_ascii=False)
    with _ws_lock:
        dead = []
        for ws in ws_clients:
            try:
                ws.send(payload)
            except Exception:
                dead.append(ws)
        for ws in dead:
            ws_clients.remove(ws)


# --- HTTP Routes ---


@app.route("/api/v1/tasks", methods=["POST"])
def create_task():
    body = request.get_json(force=True, silent=True)
    if not body:
        return _json_resp({"error": "invalid JSON body"}, 400)

    err = _validate_goal(body)
    if err:
        return _json_resp({"error": err[0]}, err[1])

    goal = _build_goal_from_body(body)
    record = TaskRecord(
        goal_id=body.get("goal_id", ""),
        goal=goal,
        error_code="",
        message="",
    )
    if not record.goal_id:
        import uuid
        record.goal_id = str(uuid.uuid4())

    task_store.add(record)
    goal_queue.put({"goal_id": record.goal_id, "target_device": body.get("target_device", "")})

    return _json_resp(
        {
            "task_id": record.goal_id,
            "status": "accepted",
            "type": goal.type,
        },
        201,
    )


@app.route("/api/v1/tasks", methods=["GET"])
def list_tasks():
    records = task_store.list_all()
    return _json_resp(
        {
            "tasks": [r.to_dict() for r in sorted(records, key=lambda r: r.created_at, reverse=True)]
        }
    )


@app.route("/api/v1/tasks/<goal_id>", methods=["GET"])
def get_task(goal_id: str):
    rec = task_store.get(goal_id)
    if rec is None:
        return _json_resp({"error": "task not found"}, 404)
    return _json_resp(rec.to_dict())


@app.route("/api/v1/tasks/<goal_id>/cancel", methods=["POST"])
def cancel_task(goal_id: str):
    rec = task_store.get(goal_id)
    if rec is None:
        return _json_resp({"error": "task not found"}, 404)
    if rec.state in ("succeeded", "failed", "canceled"):
        return _json_resp({"error": f"task already in final state: {rec.state}"}, 400)
    goal_queue.put({"goal_id": goal_id, "cancel": True})
    return _json_resp({"task_id": goal_id, "status": "cancel_accepted"})


# --- Map Routes ---


@app.route("/api/v1/map", methods=["GET"])
def get_map():
    scene = request.args.get("scene", "default")
    data = _load_map(scene)
    if data is None:
        return _json_resp({"error": f"map '{scene}' not found"}, 404)
    return _json_resp(data)


@app.route("/api/v1/map", methods=["PUT"])
def put_map():
    scene = request.args.get("scene", "default")
    body = request.get_json(force=True, silent=True)
    if not body:
        return _json_resp({"error": "invalid JSON body"}, 400)

    old = _load_map(scene)
    conflict = _check_conflicts(old, body) if old else None
    if conflict:
        return _json_resp(conflict, 409)

    _save_map(body, scene)
    return _json_resp({"status": "saved", "scene": scene})


# --- WebSocket Route ---


@sock.route("/api/v1/events")
def events(ws):
    with _ws_lock:
        ws_clients.append(ws)
    try:
        while True:
            msg = ws.receive(timeout=30)
            if msg is None:
                break
    except Exception:
        pass
    finally:
        with _ws_lock:
            if ws in ws_clients:
                ws_clients.remove(ws)
