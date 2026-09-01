from __future__ import annotations

import hashlib
import json
import os
import threading
import uuid
from typing import Any, Optional

import flask
from flask import Flask, Response, request

try:
    from flask_sock import Sock
    _sock = Sock()
    _has_ws = True
except ImportError:
    _has_ws = False

from ros_interfaces.action import ExecTask

from .diagnosis_store import DiagnosisRecord, DiagnosisStore
from .task_store import TaskRecord, TaskStore

app = Flask(__name__)
if _has_ws:
    _sock.init_app(app)

task_store: TaskStore = None
diagnosis_store: Optional[DiagnosisStore] = None
goal_queue: "queue.Queue[Optional[dict]]" = None
trigger_pub = None
maps_path: str = ""
ws_clients: list = []
_ws_lock = threading.Lock()
_api_token: str = ""


def init_app(store: TaskStore, queue: "queue.Queue[Optional[dict]]",
              maps_dir: str = "", api_token: str = "",
              diagnosis_store: Optional[DiagnosisStore] = None,
              trigger_pub=None) -> None:
    global task_store, goal_queue, maps_path, _api_token
    task_store = store
    goal_queue = queue
    maps_path = maps_dir
    _api_token = api_token
    globals()["diagnosis_store"] = diagnosis_store
    globals()["trigger_pub"] = trigger_pub


def _trace_id() -> str:
    return uuid.uuid4().hex[:16]


def _api_err(code: str, message: str, status: int = 400, details: Any = None,
             tid: str = "") -> Response:
    body = {"error": {"code": code, "message": message}}
    if details is not None:
        body["error"]["details"] = details
    if tid:
        body["trace_id"] = tid
    resp = flask.Response(
        json.dumps(body, ensure_ascii=False),
        status=status,
        content_type="application/json",
    )
    if tid:
        resp.headers["X-Trace-Id"] = tid
    return resp


def _ok_resp(data: Any, status: int = 200, tid: str = "",
             headers: dict = None) -> Response:
    if isinstance(data, dict) and tid:
        data["trace_id"] = tid
    resp = flask.Response(
        json.dumps(data, ensure_ascii=False),
        status=status,
        content_type="application/json",
    )
    if tid:
        resp.headers["X-Trace-Id"] = tid
    if headers:
        for k, v in headers.items():
            resp.headers[k] = v
    return resp


def _check_auth(tid: str) -> Optional[Response]:
    if not _api_token:
        return None
    token = request.headers.get("X-API-Key", "")
    if token != _api_token:
        return _api_err("UNAUTHORIZED", "invalid or missing X-API-Key", 401, tid=tid)


def _load_map(scene: str = "default") -> Optional[dict]:
    filepath = os.path.join(maps_path, f"{scene}.json")
    if not os.path.exists(filepath):
        return None
    with open(filepath, "r") as f:
        return json.load(f)


def _load_map_etag(scene: str = "default") -> Optional[tuple[dict, str]]:
    filepath = os.path.join(maps_path, f"{scene}.json")
    if not os.path.exists(filepath):
        return None
    with open(filepath, "rb") as f:
        data = f.read()
    etag = hashlib.md5(data).hexdigest()
    return (json.loads(data), etag)


def _save_map(data: dict, scene: str = "default") -> None:
    filepath = os.path.join(maps_path, f"{scene}.json")
    with open(filepath, "w") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)


def _validate_goal(body: dict) -> Optional[tuple[str, str, int]]:
    goal = body.get("goal")
    if not goal:
        return ("INVALID_GOAL", "missing goal field", 400)
    if not goal.get("type"):
        return ("INVALID_GOAL", "missing goal.type", 400)
    valid_types = {"go_to_tag", "patrol_route", "hold"}
    if goal["type"] not in valid_types:
        return ("INVALID_GOAL", f"type must be one of {valid_types}", 400)
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
        "code": "CONFLICT",
        "message": "cannot delete items used by active task(s)",
        "details": {
            "deleted_tags": sorted(deleted_tags),
            "deleted_edges": [{"from": e[0], "to": e[1]} for e in deleted_edges],
            "deleted_routes": sorted(deleted_routes),
            "blocking_tasks": blocking,
        },
    }


def broadcast(goal_id: str, event_type: str, data: dict) -> None:
    payload = json.dumps({"goal_id": goal_id, "event": event_type, **data}, ensure_ascii=False)
    _push_ws(payload)


def broadcast_diagnosis(payload: dict) -> None:
    """Broadcast a `diagnosis` WS event (RFC-009 §5.4)."""
    payload = dict(payload, event="diagnosis")
    _push_ws(json.dumps(payload, ensure_ascii=False))


def _push_ws(data: str) -> None:
    if not _has_ws:
        return
    with _ws_lock:
        dead = []
        for ws in ws_clients:
            try:
                ws.send(data)
            except Exception:
                dead.append(ws)
        for ws in dead:
            ws_clients.remove(ws)


# --- Before Request ---


@app.before_request
def _attach_trace_id():
    tid = request.headers.get("X-Trace-Id", "")
    if not tid:
        tid = uuid.uuid4().hex[:16]
    request.trace_id = tid


# --- HTTP Routes ---


@app.route("/api/v1/tasks", methods=["POST"])
def create_task():
    tid = request.trace_id
    auth_err = _check_auth(tid)
    if auth_err:
        return auth_err
    body = request.get_json(force=True, silent=True)
    if not body:
        return _api_err("INVALID_JSON", "request body is not valid JSON", 400, tid=tid)

    err = _validate_goal(body)
    if err:
        return _api_err(err[0], err[1], err[2], tid=tid)

    goal = _build_goal_from_body(body)
    record = TaskRecord(
        goal_id=body.get("goal_id", ""),
        goal=goal,
        error_code="",
        message="",
    )
    if not record.goal_id:
        record.goal_id = str(uuid.uuid4())

    task_store.add(record)
    goal_queue.put({"goal_id": record.goal_id, "target_device": body.get("target_device", "")})

    return _ok_resp(
        {
            "task_id": record.goal_id,
            "status": "accepted",
            "type": goal.type,
        },
        201, tid=tid,
    )


@app.route("/api/v1/tasks", methods=["GET"])
def list_tasks():
    tid = request.trace_id
    auth_err = _check_auth(tid)
    if auth_err:
        return auth_err
    records = task_store.list_all()
    records.sort(key=lambda r: r.created_at, reverse=True)
    total = len(records)
    try:
        offset = max(0, int(request.args.get("offset", "0")))
        limit = max(1, min(200, int(request.args.get("limit", "50"))))
    except ValueError:
        return _api_err("INVALID_PARAM", "offset and limit must be integers", 400, tid=tid)
    page = records[offset:offset + limit]
    return _ok_resp(
        {
            "tasks": [r.to_dict() for r in page],
            "total": total,
            "offset": offset,
            "limit": limit,
        },
        tid=tid,
    )


@app.route("/api/v1/tasks/<goal_id>", methods=["GET"])
def get_task(goal_id: str):
    tid = request.trace_id
    auth_err = _check_auth(tid)
    if auth_err:
        return auth_err
    rec = task_store.get(goal_id)
    if rec is None:
        return _api_err("NOT_FOUND", f"task {goal_id} not found", 404, tid=tid)
    return _ok_resp(rec.to_dict(), tid=tid)


@app.route("/api/v1/tasks/<goal_id>/cancel", methods=["POST"])
def cancel_task(goal_id: str):
    tid = request.trace_id
    auth_err = _check_auth(tid)
    if auth_err:
        return auth_err
    rec = task_store.get(goal_id)
    if rec is None:
        return _api_err("NOT_FOUND", f"task {goal_id} not found", 404, tid=tid)
    if rec.state in ("succeeded", "failed", "canceled"):
        return _api_err("INVALID_STATE", f"task already in final state: {rec.state}", 400, tid=tid)
    goal_queue.put({"goal_id": goal_id, "cancel": True})
    return _ok_resp({"task_id": goal_id, "status": "cancel_accepted"}, tid=tid)


# --- Map Routes ---


@app.route("/api/v1/map", methods=["GET"])
def get_map():
    tid = request.trace_id
    auth_err = _check_auth(tid)
    if auth_err:
        return auth_err
    scene = request.args.get("scene", "default")
    result = _load_map_etag(scene)
    if result is None:
        return _api_err("NOT_FOUND", f"map '{scene}' not found", 404, tid=tid)

    data, etag = result
    if request.headers.get("If-None-Match", "") == etag:
        resp = flask.Response(status=304)
        if tid:
            resp.headers["X-Trace-Id"] = tid
        return resp

    return _ok_resp(data, headers={"ETag": etag}, tid=tid)


@app.route("/api/v1/map", methods=["PUT"])
def put_map():
    tid = request.trace_id
    auth_err = _check_auth(tid)
    if auth_err:
        return auth_err
    scene = request.args.get("scene", "default")
    body = request.get_json(force=True, silent=True)
    if not body:
        return _api_err("INVALID_JSON", "request body is not valid JSON", 400, tid=tid)

    old = _load_map(scene)
    if old:
        conflict = _check_conflicts(old, body)
        if conflict:
            return _api_err(conflict["code"], conflict["message"], 409,
                            details=conflict["details"], tid=tid)

    _save_map(body, scene)
    _, etag = _load_map_etag(scene) or ("", "")
    return _ok_resp({"status": "saved", "scene": scene}, headers={"ETag": etag}, tid=tid)


# --- Diagnostics Routes (RFC-009) ---


@app.route("/api/v1/diagnostics", methods=["POST"])
def trigger_diagnosis():
    tid = request.trace_id
    auth_err = _check_auth(tid)
    if auth_err:
        return auth_err
    if diagnosis_store is None or trigger_pub is None:
        return _api_err("INTERNAL", "diagnosis layer not available", 503, tid=tid)
    from std_msgs.msg import String as StringMsg
    trigger_pub.publish(StringMsg(data=f"manual:{tid}"))
    return _ok_resp({"status": "triggered", "trigger_type": "manual"}, 202, tid=tid)


@app.route("/api/v1/diagnostics", methods=["GET"])
def list_diagnoses():
    tid = request.trace_id
    auth_err = _check_auth(tid)
    if auth_err:
        return auth_err
    if diagnosis_store is None:
        return _api_err("INTERNAL", "diagnosis store not available", 503, tid=tid)
    try:
        offset = max(0, int(request.args.get("offset", "0")))
        limit = max(1, min(200, int(request.args.get("limit", "50"))))
    except ValueError:
        return _api_err("INVALID_PARAM", "offset and limit must be integers", 400, tid=tid)
    records = diagnosis_store.list_all(offset=offset, limit=limit)
    total = diagnosis_store.count()
    return _ok_resp(
        {
            "diagnoses": [r.to_dict() for r in records],
            "total": total,
            "offset": offset,
            "limit": limit,
        },
        tid=tid,
    )


@app.route("/api/v1/diagnostics/<diagnosis_id>", methods=["GET"])
def get_diagnosis(diagnosis_id: str):
    tid = request.trace_id
    auth_err = _check_auth(tid)
    if auth_err:
        return auth_err
    if diagnosis_store is None:
        return _api_err("INTERNAL", "diagnosis store not available", 503, tid=tid)
    rec = diagnosis_store.get(diagnosis_id)
    if rec is None:
        return _api_err("NOT_FOUND", f"diagnosis {diagnosis_id} not found", 404, tid=tid)
    return _ok_resp(rec.to_dict(), tid=tid)


@app.route("/api/v1/internal/diagnosis-result", methods=["POST"])
def receive_diagnosis_result():
    try:
        body = request.get_json(force=True, silent=True) or {}
        rec = DiagnosisRecord(
            diagnosis_id=body.get("diagnosis_id", ""),
            source_ids=body.get("source_ids", []),
            trigger_type=body.get("trigger_type", ""),
            severity=body.get("severity", "normal"),
            summary=body.get("summary", ""),
            possible_causes=body.get("possible_causes", []),
            recommendations=body.get("recommendations", []),
            confidence=float(body.get("confidence", 0.0)),
            disclaimer=body.get("disclaimer", ""),
            raw_prompt=body.get("raw_prompt", ""),
            error_code=body.get("error_code", ""),
            error_message=body.get("error_message", ""),
        )
        diagnosis_store.add(rec)
        threading.Thread(target=broadcast_diagnosis, args=(rec.to_dict(),), daemon=True).start()
        return _ok_resp({"status": "stored", "diagnosis_id": rec.diagnosis_id}, 201)
    except Exception as e:
        import traceback, sys
        traceback.print_exc(file=sys.stderr)
        sys.stderr.flush()
        return _api_err("INTERNAL", f"receive failed: {e}", 500)


# --- WebSocket Route (optional) ---


if _has_ws:
    @_sock.route("/api/v1/events")
    def events(ws):
        tid = uuid.uuid4().hex[:16]
        if _api_token:
            token = ws.receive(timeout=5)
            if token != _api_token:
                ws.send(json.dumps({"error": {"code": "UNAUTHORIZED", "message": "auth failed"}}))
                return
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
