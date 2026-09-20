# Align WebUI Task API Implementation Plan

> **For agentic workers:** Execute this plan in the current session. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the obsolete WebUI task model with the Gateway's canonical task API and remove all frontend behavior unsupported by the backend.

**Architecture:** The WebUI uses the backend task contract directly: `CreateTask` contains `device_id`, `primitive`, `target`, and `deadline_ms`; responses are `TaskView`; task events are `{ sequence, event }`. There is no client-side adapter, pagination contract, cancellation flow, legacy task type, or unsupported task detail field.

**Tech Stack:** React, TypeScript, Vite, native `fetch`, WebSocket.

**Spec:** `ros2_ws/src/services/gateway/src/dto.rs`, `ros2_ws/src/services/gateway/src/lib.rs`, and `ros2_ws/src/services/platform/src/task.rs`.

## Global Constraints

- Do not modify backend code.
- Do not add frontend compatibility layers or support the old `{ target_device, goal }` request shape.
- Only `go_to_tag` is supported.
- A task has only `id`, `device_id`, `primitive`, `target`, `deadline_ms`, `state`, `progress`, and `phase` in the UI model.
- Do not retain pagination, cancellation, priority, route metadata, current/next tag fields, error-code fields, or unsupported task types without backend support.
- Do not add tests; validate with the existing WebUI build command.

---

### Task 1: Replace frontend task contracts and API functions

**Files:**
- Modify: `webui/src/types/task.ts`
- Modify: `webui/src/api/tasks.ts`

**Interfaces:**

```ts
type Primitive = "go_to_tag"

interface Task {
  id: string
  device_id: string
  primitive: Primitive
  target: number[]
  deadline_ms: number | null
}

interface TaskView {
  task: Task
  state: "accepted" | "running" | "succeeded" | "failed"
  progress: number
  phase: string
}
```

- [ ] Replace old `TaskGoal`, `TaskRecord`, `WsMessage`, cancellation, pagination, and unsupported primitive types with the backend types.
- [ ] Make `createTask(deviceId, target, deadlineMs)` send exactly `{ device_id, primitive: "go_to_tag", target, deadline_ms }`.
- [ ] Make `listTasks()` return `Promise<TaskView[]>` from `GET /api/v1/tasks`.
- [ ] Make `getTask(taskId)` return `Promise<TaskView>`.
- [ ] Delete `cancelTask` and all old request/response shapes.

---

### Task 2: Align task creation UI

**Files:**
- Modify: `webui/src/pages/TaskNew.tsx`

- [ ] Remove task type selection and hold/patrol options.
- [ ] Add required device ID input.
- [ ] Keep ordered Tag input and parse it into `number[]`.
- [ ] Reject an empty Tag list before calling the API.
- [ ] Add the optional deadline input only if it maps directly to the backend's absolute `deadline_ms` field; do not send a relative value under the same name.
- [ ] Call the new `createTask` signature and keep existing loading/error/toast behavior.

---

### Task 3: Align list/detail views and remove unsupported actions

**Files:**
- Modify: `webui/src/pages/TaskList.tsx`
- Modify: `webui/src/pages/TaskDetail.tsx`
- Modify: `webui/src/components/TaskStatusBadge.tsx` only if its state union requires adjustment.
- Delete: `webui/src/components/RouteMap.tsx` if no remaining supported data can populate it.

- [ ] Remove offset/limit state, load-more behavior, total-count assumptions, and old `goal_id` access.
- [ ] Render `TaskView.task.id`, `task.device_id`, `task.target`, `state`, `progress`, and `phase`.
- [ ] Remove cancel button, cancel state, cancel API import, route metadata, current/next tag, error code, final state, and finished-stage rendering.
- [ ] Remove RouteMap usage and delete the component only if repository-wide search confirms no other consumer.
- [ ] Keep task detail fetch and not-found/loading states.

---

### Task 4: Align WebSocket events and application state

**Files:**
- Modify: `webui/src/hooks/useTaskWS.ts`
- Modify: `webui/src/App.tsx`
- Modify: `webui/src/types/task.ts` if event types are defined there.

**Interfaces:**

```ts
type TaskEvent = {
  sequence: number
  event: {
    TaskStateChanged: {
      task_id: string
      state: TaskState
    }
  }
}
```

- [ ] Parse the backend envelope `{ sequence, event }` instead of the old `{ goal_id, event, ... }` message.
- [ ] Update only the matching task's `state`; do not invent progress/details fields that the event does not carry.
- [ ] Remove old `Record<string, Partial<TaskRecord>>` assumptions and any unsupported event fields.
- [ ] Preserve reconnect behavior and existing page navigation.

---

### Task 5: Remove stale frontend API surface and verify

**Files:**
- Search and modify all remaining `webui/src` consumers of removed task fields/functions.
- Remove unused imports and components after the contract migration.

- [ ] Search for `target_device`, `goal`, `goal_id`, `TaskGoal`, `TaskRecord`, `cancelTask`, `target_tags`, `current_tag`, `next_tag`, `route_id`, `constraints`, `priority`, `patrol_route`, and `hold`.
- [ ] Delete stale code rather than preserving dead compatibility paths.
- [ ] Remove unsupported diagnosis, vitals, and map pages, APIs, hooks, types, and components.
- [ ] Remove dependencies used only by those deleted pages.
- [ ] Run `pnpm build` from `webui`.
- [ ] Run `git diff --check` and confirm no backend files changed.
