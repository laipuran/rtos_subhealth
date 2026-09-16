# TonyPi Python Exec Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or **superpowers:executing-plans** to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 RPi4B 的 Ubuntu 22.04 + ROS 2 Humble 上实现独立的 Python `tonypi-exec`，通过现有设备无关 ROS2 契约调用 TonyPi Python SDK。

**Architecture:** Python exec 分为设备无关的执行运行时、设备 backend 接口和 TonyPi SDK backend。运行时只依赖 backend 接口，不导入 TonyPi SDK；只有组合根和 `TonyPiBackend` 依赖厂商 SDK。服务提供 `/<device_id>/device_task` action，并发布 `/device_descriptors` 和 `/device_states`。

**Tech Stack:** Python 3、ROS 2 Humble、`rclpy`、现有 `device_interfaces`/`task_interfaces` rosidl 包、TonyPi Python SDK、pytest、systemd。

**Spec:** `docs/rfc/rfc-011-tonypi-exec.md`

## 全局约束

- 不修改任何 `.msg`、`.action` 或 `.srv` 文件。
- 不在 TonyPi exec 中运行 WebUI、gateway、orchestrator 或全局 planner。
- 不让 `orchestrator`、`gateway` 或通用 exec 导入 TonyPi SDK。
- TonyPi 默认只声明 `execute_primitive`、`hold`、`stop`；不声明 `set_velocity`、`move_to_pose`、odometry 或 pose feedback。
- ROS action 名称为 `/<device_id>/device_task`；descriptor 使用 transient-local QoS。
- SDK 调用失败必须转换为统一的 `error_code` 和 `message`。
- 所有测试必须能在没有真实 TonyPi 硬件的环境运行。
- 执行期间必须有明确的 deadline、取消和本地停止路径；服务退出时必须尝试调用 SDK stop。

---

### Task 3：实现 TonyPi SDK backend

**Files:**
- Create: `robot-endpoints/tonypi-exec/tonypi_exec/tonypi_backend.py`
- Create: `robot-endpoints/tonypi-exec/tonypi_exec/vendor.py`
- Create: `robot-endpoints/tonypi-exec/tests/test_tonypi_backend.py`

**Interfaces:**
- `VendorTonyPi` Protocol：`run_action_group(name, repeat)`、`stop()`、`read_state()`。
- `TonyPiBackend(vendor)`：实现 Task 2 的 `Backend` Protocol。

- [ ] **Step 1: 写 backend 失败测试**

使用 `FakeVendorTonyPi` 验证 descriptor 仅包含 `supports_action_groups`、`execute_primitive`、`hold`、`stop`；验证动作组参数、hold、stop、非法 action 和 SDK 异常。

- [ ] **Step 2: 运行测试确认失败**

运行：`python3 -m pytest tests/test_tonypi_backend.py -q`

- [ ] **Step 3: 实现 backend 和 SDK 包装**

实现以下映射：

```text
execute_primitive + {"action": "wave", "repeat": 2}
  → vendor.run_action_group("wave", repeat=2)
hold → vendor.run_action_group("stand")
stop → vendor.stop()
```

厂商 SDK 的 import 只放在 `vendor.py` 的初始化路径。SDK 不存在时返回明确的 `SDK_UNAVAILABLE`，不发布虚假的 healthy 状态。

- [ ] **Step 4: 运行测试确认通过**

运行：`python3 -m pytest tests/test_tonypi_backend.py tests/test_runtime.py -q`

- [ ] **Step 5: 提交**

```bash
git add robot-endpoints/tonypi-exec/tonypi_exec/tonypi_backend.py robot-endpoints/tonypi-exec/tonypi_exec/vendor.py robot-endpoints/tonypi-exec/tests/test_tonypi_backend.py
git commit -m "feat(tonypi): map python sdk through backend"
```

---

### Task 4：实现 ROS2 Humble action/topic 节点

**Files:**
- Create: `robot-endpoints/tonypi-exec/tonypi_exec/node.py`
- Create: `robot-endpoints/tonypi-exec/tests/test_node_mapping.py`
- Modify: `robot-endpoints/tonypi-exec/package.xml`
- Modify: `robot-endpoints/tonypi-exec/pyproject.toml`

**Interfaces:**
- 提供 `/<device_id>/device_task`，类型为 `task_interfaces/action/DeviceTask`。
- 发布 `/device_descriptors`，类型为 `device_interfaces/msg/DeviceDescriptor`，使用 transient-local。
- 发布 `/device_states`，类型为 `device_interfaces/msg/DeviceState`。
- 使用 Task 2 的 `ExecutionRuntime` 和 Task 3 的 `TonyPiBackend`。

- [ ] **Step 1: 写 payload 映射测试**

覆盖 descriptor、`params_json` 严格解析、非法 JSON、无 pose 时不伪造 pose feedback。

- [ ] **Step 2: 运行测试确认失败**

运行：`python3 -m pytest tests/test_node_mapping.py -q`

- [ ] **Step 3: 实现节点**

启动时读取 `DEVICE_ID`，初始化 vendor/backend/runtime，发布 descriptor，启动 action server 和 100 ms state timer。SDK 初始化失败时退出并交给 systemd 重启。action 执行交给 worker thread，避免阻塞 `rclpy` executor。

执行过程中轮询 `goal_handle.is_cancel_requested`，将取消交给 runtime；发布 `running` feedback 和最终 result。

- [ ] **Step 4: 运行包级测试**

运行：`python3 -m pytest -q`

- [ ] **Step 5: 在 Humble 环境构建**

```bash
source /opt/ros/humble/setup.bash
colcon build --packages-select tonypi_exec
```

- [ ] **Step 6: 提交**

```bash
git add robot-endpoints/tonypi-exec
git commit -m "feat(tonypi): expose python exec over ros2"
```

---

### Task 5：添加 RPi4B 配置和 systemd 服务

**Files:**
- Create: `robot-endpoints/tonypi-exec/config/tonypi-exec.env.example`
- Create: `robot-endpoints/tonypi-exec/deploy/tonypi-exec.service`
- Modify: `deploy/README.md`
- Modify: `docs/guide/getting-started.md`

**Interfaces:**
- 环境变量：`DEVICE_ID`、`TONYPI_SDK_ROOT`、`ROS_DOMAIN_ID`、`RMW_IMPLEMENTATION`、`TONYPI_ACTION_TIMEOUT_S`。
- 服务名称：`tonypi-exec.service`。

- [ ] **Step 1: 写部署检查**

检查 service 包含 `After=network-online.target`、`Restart=on-failure`、可选 `EnvironmentFile` 和正确的 Python entry point；运行用户必须具备访问 SDK/硬件所需权限。

- [ ] **Step 2: 实现配置和 service**

service 启动前 source ROS 2 Humble 环境以及 endpoint workspace，不依赖控制电脑 Docker 镜像。

- [ ] **Step 3: 在 RPi4B 上验证启动**

```bash
systemctl enable --now tonypi-exec
systemctl status tonypi-exec
ros2 topic echo /device_descriptors --qos-durability transient_local
ros2 topic echo /device_states
```

- [ ] **Step 4: 提交**

```bash
git add robot-endpoints/tonypi-exec/config robot-endpoints/tonypi-exec/deploy deploy/README.md docs/guide/getting-started.md
git commit -m "deploy(tonypi): add endpoint systemd service"
```

---

### Task 6：控制电脑端到端验证和旧路径隔离

**Files:**
- Modify: `README.md`
- Modify: `docs/rfc/rfc-011-tonypi-exec.md`
- Modify: `deploy/config/adapter-tonypi.env`，仅标注为兼容/模拟路径
- Create: `robot-endpoints/tonypi-exec/tests/test_contract_fixture.py`

**Interfaces:**
- Consumes: RPi4B 发布的 descriptor 和 device action server。
- Produces: 实机验证记录，证明 Rust orchestrator 不依赖 Python exec 的实现细节。

- [ ] **Step 1: 写 descriptor fixture 测试**

验证 TonyPi descriptor 接受 `execute_primitive`、`hold`、`stop`，拒绝 `move_to_pose` 和 `set_velocity`。

- [ ] **Step 2: 运行测试并补充 fixture**

运行：`python3 -m pytest tests/test_contract_fixture.py -q`

- [ ] **Step 3: 运行控制电脑软件链路**

使用 Python fake endpoint 验证：

```text
POST /api/v1/tasks
  → gateway_bridge
  → orchestrator
  → /tonypi/device_task
  → fake backend
  → feedback/result
```

- [ ] **Step 4: 运行 RPi4B 实机验收**

至少验证动作组、hold、stop、取消、deadline、SDK 异常、DDS 断开后的安全行为和 RPi4B 重启恢复。

- [ ] **Step 5: 提交验证记录**

```bash
git add README.md docs/rfc/rfc-011-tonypi-exec.md deploy/config/adapter-tonypi.env robot-endpoints/tonypi-exec/tests/test_contract_fixture.py
git commit -m "test(tonypi): verify python endpoint contract"
```

## 完成标准

- Python `tonypi-exec` 可在 RPi4B/Humble 独立启动。
- 控制电脑只依赖 `DeviceDescriptor` 和 `DeviceTask`，不依赖 Python SDK。
- TonyPi SDK 只被 `vendor.py`/`TonyPiBackend` 使用。
- `execute_primitive`、`hold`、`stop`、取消和 deadline 都有测试和实机路径。
- SDK 不可用或异常时，服务不会发布虚假的 healthy 状态。
- 当前 Rust `DEVICE_TYPE=tonypi` 路径被明确标记为兼容路径，不与 Python endpoint 混淆。

### Task 1：建立 Python endpoint 包和测试边界

**Files:**
- Create: `robot-endpoints/tonypi-exec/pyproject.toml`
- Create: `robot-endpoints/tonypi-exec/package.xml`
- Create: `robot-endpoints/tonypi-exec/tonypi_exec/__init__.py`
- Create: `robot-endpoints/tonypi-exec/tests/__init__.py`
- Create: `robot-endpoints/tonypi-exec/tests/test_package_contract.py`
- Modify: `docs/guide/getting-started.md`

**Interfaces:**
- Produces: 可被 `colcon build` 构建的 `ament_python` 包；Python 单元测试不要求 ROS 或 TonyPi SDK。

- [ ] **Step 1: 写包契约测试**

测试包元数据存在，并确保核心模块可以在不导入 `rclpy` 和厂商 SDK 的情况下导入：

```python
def test_runtime_import_does_not_require_vendor_sdk():
    import tonypi_exec.runtime
    import tonypi_exec.backend
```

- [ ] **Step 2: 运行测试确认失败**

运行：`cd robot-endpoints/tonypi-exec && python3 -m pytest -q`

预期：因包文件和模块尚不存在而失败。

- [ ] **Step 3: 创建最小包骨架**

配置 `ament_python` entry point，但不要在包导入时初始化 `rclpy` 或 TonyPi SDK。

- [ ] **Step 4: 运行测试确认通过**

运行同一 pytest 命令，预期全部通过。

- [ ] **Step 5: 提交**

```bash
git add robot-endpoints/tonypi-exec docs/guide/getting-started.md
git commit -m "feat(tonypi): add python endpoint package skeleton"
```

---

### Task 2：实现设备无关 backend 接口和执行模型

**Files:**
- Create: `robot-endpoints/tonypi-exec/tonypi_exec/model.py`
- Create: `robot-endpoints/tonypi-exec/tonypi_exec/backend.py`
- Create: `robot-endpoints/tonypi-exec/tonypi_exec/runtime.py`
- Create: `robot-endpoints/tonypi-exec/tests/test_runtime.py`

**Interfaces:**
- `BackendDescriptor`：设备描述、能力、primitive、sensor 和 limits。
- `BackendState`：`healthy`、`mode`、`error_code`、可选 pose/velocity。
- `Backend` Protocol：`descriptor()`、`execute(primitive, params)`、`hold()`、`stop()`、`state()`。
- `ExecutionRuntime.execute(goal, is_cancel_requested)`：返回统一 `ExecutionResult`，负责 primitive 检查、deadline、取消和异常转换。

- [ ] **Step 1: 写失败测试**

使用 `FakeBackend` 覆盖：不支持 primitive、动作组参数、deadline 超时、取消停止、backend 异常。

- [ ] **Step 2: 运行测试确认失败**

运行：`python3 -m pytest tests/test_runtime.py -q`

- [ ] **Step 3: 实现最小运行时**

实现以下行为：primitive 不在 descriptor 中时返回 `UNSUPPORTED_CAPABILITY`；deadline 到期先调用 `backend.stop()` 再返回 `TIMEOUT`；取消时调用 `backend.stop()` 并返回 `CANCELED`；backend 异常返回 `INTERNAL`。

- [ ] **Step 4: 运行测试确认通过**

运行：`python3 -m pytest tests/test_runtime.py -q`

- [ ] **Step 5: 提交**

```bash
git add robot-endpoints/tonypi-exec/tonypi_exec robot-endpoints/tonypi-exec/tests/test_runtime.py
git commit -m "feat(tonypi): add device-independent execution runtime"
```

---
