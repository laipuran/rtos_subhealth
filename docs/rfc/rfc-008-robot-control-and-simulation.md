## RFC 008: 执行层机器人控制与仿真对接

**状态：** 草案

**修订日期：** 2026-07-21

---

## 1. 摘要

当前 exec_layer 的 `_drive_segment()` 是空桩，无法驱动机器人运动；无模拟环境，全链路验证依赖真实机器人。本 RFC 定义 exec_layer 通过 unitree SportClient 下发速度指令驱动机器人、配合 AprilTag 检测完成 segment 到达闭环的方案，以及 MuJoCo 仿真与真机 GO2 共用同一套调用接口的切换方式。

---

## 2. 目标与非目标

**目标：**
1. 实现 `_drive_segment(from_tag, to_tag)` 的真实运动逻辑：SportClient.Move + AprilTag 反馈闭环。
2. 一套 exec_layer 代码同时支持 MuJoCo 仿真与真机 GO2，通过参数/环境切换。
3. 明确 MuJoCo 仿真启动后的 DDS 桥接机制，使 SportClient 调用无感转发到仿真。
4. SportClient 调用失败时 exec_layer 上报对应 error_code 并终止任务。

**非目标：**
1. 不定义 Tag Graph 的数据格式与编辑方式（见 RFC 007）。
2. 不定义 AprilTag 感知发布协议（见 RFC 001）。
3. 不实现底层电机控制与姿态调整动作组（见 RFC 002）。
4. 不替换 mock_exec_layer（两者共存，mock 用于纯 WebUI 验证）。

---

## 3. 现状与痛点

1. `exec_layer/_drive_segment()` 仅为日志空桩，无法连接机器人硬件，WebUI 看不到真实运动。
2. 无模拟环境，每次验证需要启动真机 GO2，迭代效率低、存在安全风险。
3. exec_layer 不消费 `/perception/apriltag_detections` topic，无法利用感知反馈做到达判定。
4. 仿真与真机的控制代码分离，维护两套实现成本高。

---

## 4. 方案概览

exec_layer 引入 unitree `SportClient` 作为机器人控制接口，`_drive_segment()` 内循环执行：调用 `Move()` 发速度指令 → 订阅 `/apriltag_detections` 检查目标 tag 是否到达 → 到达后 `Damp()` 停止 → 更新 feedback。MuJoCo 仿真通过 `unitree_sdk2py_bridge` 自动创建 DDS 通道，接收 SportClient 的 `rt/lowcmd`/`rt/sportmodestate` 请求并映射到 MuJoCo 物理仿真。exec_layer 不区分下层是模拟还是真机，调用同一套 API。

```
                        Exec Layer
                   ┌──────────────────┐
                   │ _drive_segment() │
                   │  for s in segs:  │
                   │    Move(vx,vy)   │
                   │    wait tag_det  │
                   │    Damp()        │
                   │    pub feedback  │
                   └───────┬──────────┘
                           │ SportClient API
                           │ (DDS rt/lowcmd / rt/sportmodestate)
                           ▼
              ┌─────────────────────────┐
              │     DDS 层 (Domain 1)    │
              └────────┬────────┬────────┘
                       │        │
              ┌────────▼─┐  ┌──▼─────────┐
              │ unitree   │  │  GO2 实机  │
              │ MuJoCo    │  │  自带 DDS  │
              │ bridge    │  │  通信      │
              └───────────┘  └────────────┘
                       │
              ┌────────▼────────┐
              │  MuJoCo 仿真     │
              │  mj_step()      │
              └─────────────────┘
```

---

## 5. 关键接口

### 5.1 机器人控制接口（unitree SportClient）

| 调用 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `SportClient.Move(vx, vy, vyaw)` | float32 ×3 | — | 速度控制 (m/s, m/s, rad/s) |
| `SportClient.StandUp()` | — | — | 从躺卧到站立 |
| `SportClient.Damp()` | — | — | 停止运动并进入阻尼模式 |
| `SportClient.RecoveryStand()` | — | — | 跌倒后恢复站立 |
| `SportClient.BalanceStand()` | — | — | 平衡站立（保持位置） |

`SportClient` 初始化需 `ChannelFactoryInitialize(domain_id, interface)`，参数通过 ROS2 参数传入 exec_layer。

### 5.2 ROS2 接口

| 类型 | 名称 | 消息 | 说明 |
|---|---|---|---|
| Action | `/exec_task` | `ExecTask.action` | 不变，WebUI → Desc → Exec 单一入口 |
| Topic | `/perception/apriltag_detections` | `AprilTagDetections.msg` | 订阅感知输出，用于到达判定 |
| Action | `/mock_exec_task` | `ExecTask.action` | Mock 模式用，不与 SportClient 冲突 |

### 5.3 模拟 / 真机切换

| 模式 | 启动方式 | SportClient 目标 | AprilTag 来源 |
|---|---|---|---|
| Mock | `ros2 run mock_exec_layer` | 不使用 | 无（内置定时器模拟） |
| Simulation | 先启动 `unitree_mujoco.py`，再启动 exec_layer | MuJoCo bridge 消费 DDS | 仿真相机 + apriltag 节点 |
| Real Robot | `ros2 run exec_layer` | GO2 实机 DDS | 实机相机 + apriltag 节点 |

exec_layer 通过参数 `use_sport_client`（默认 false）控制是否启用 SportClient。为 false 时退化为当前 stub 行为（兼容 mock）。

---

## 6. 数据/控制流

### 6.1 Segment 执行闭环

```
_drive_segment(from_tag, to_tag):
  → 从 tag_graph.json 查 from→to 位移向量 (dx, dy)
  → SportClient.StandUp()   // 确保站立
  → 计算速度: vx = dx / estimate_t, vy = dy / estimate_t
  → SportClient.Move(vx, vy, 0)
  → 进入到达判定循环（每 100ms 检查一次）:
      → 从 /apriltag_detections 获取最新检测
      → 在 detections[] 中查找 id == to_tag 的条目
      → 若找到:
          → center_offset_x 绝对值 < 5%
          → center_offset_y 绝对值 < 5%
          → distance < 500mm
          → 满足以上全部 → 判定到达
          → 循环结束
      → 若超时（deadline_ms 耗尽）:
          → 上报 error_code = TIMEOUT
          → 循环结束
  → SportClient.Damp()
  → 更新 feedback(current_tag=to_tag, progress+=1/N)
```

### 6.2 速度计算

`_drive_segment` 从 Tag Graph 获取 from→to 的 `(x, y)` 坐标差，换算为机器人坐标系下的速度指令：

```
dx = tags[to].x - tags[from].x
dy = tags[to].y - tags[from].y
estimate_t = sqrt(dx² + dy²) / default_speed
vx = dx / estimate_t
vy = dy / estimate_t
```

`default_speed` 从 goal 的 `constraints.max_speed_mps` 获取，未设置时默认 0.3 m/s。

### 6.3 MuJoCo 仿真 DDS 桥接

`unitree_mujoco/simulate_python/unitree_mujoco.py` 启动时执行：

```
ChannelFactoryInitialize(domain_id=1, interface="lo")
UnitreeSdk2Bridge(model, data)
  → 订阅 DDS topic "rt/lowcmd"
  → 将低阶控制指令映射到 MuJoCo 电机
  → 发布 DDS topic "rt/sportmodestate"（含位置/速度/IMU）
```

SportClient 的 `Move()` 内部向 `rt/sportmodestate` 等 topic 发布指令，向 `unitree_api` service 发送请求。MuJoCo bridge 消费这些指令后驱动仿真。

### 6.4 错误码与异常处理

| 场景 | error_code | 行为 |
|---|---|---|
| SportClient 初始化失败 | `INTERNAL` | `goal_handle.abort()`，任务终止 |
| 目标 tag 在检测中持续缺失 | `UNREACHABLE` | 超时后终止，robot 原地 Damp |
| deadline_ms 耗尽 | `TIMEOUT` | 同 UNREACHABLE |
| 机器人跌倒（通过状态检测） | `INTERNAL` | 尝试 RecoveryStand，失败则终止 |
| cancel 请求 | — | 立即 Damp()，返回 canceled |

---

## 7. 风险与替代

1. **替代方案：** 用低阶 `LowCmd` 直接控制电机，而非高阶 SportClient。
   - **权衡：** 控制粒度更细可做平滑插值，但需要逆运动学与状态机，实现复杂度大幅增加。
2. **替代方案：** MuJoCo 仿真打包为 ROS2 `ament_python` 节点而非独立脚本。
   - **权衡：** 纳入 colcon 管理便于 launch 集成，但 MuJoCo 的 viewer 在主线程阻塞，与 rclpy spin 冲突。
3. **风险：** MuJoCo 仿真与真机的运动动力学存在差异，`Move()` 速度参数调优不能直接复用。
4. **风险：** AprilTag 发布频率低于 10Hz 时，segment 到达判定可能延迟，需加降级逻辑。

---

## 8. 验证计划

| 测试 | 方法 | 通过标准 |
|---|---|---|
| StandUp/Move/Damp 调用 | exec_layer 启动后手动调用 SportClient 方法 | 机器人（仿真或实机）执行对应动作 |
| 仿真 Segment 执行 | MuJoCo + exec_layer，下发 go_to_tag(42) | 机器人在仿真中朝 tag 移动 |
| AprilTag 闭环 | 仿真中 tag 置于机器人前方 | 机器人移动到 tag 前停止，current_tag 更新 |
| 超时终止 | deadline_ms 设为 1s，tag 不在视野 | error_code = TIMEOUT，任务 failed |
| Cancel | WS /cancel | 机器人立即暂停，state = canceled |
| Mock 兼容 | `use_sport_client=false` 执行 | 退化为当前 stub 行为 |

---

## 9. 未决问题

1. MuJoCo 仿真启动是否打包为 ROS2 `ament_python` 包，还是保持独立 Python 脚本 + 子进程启动。
2. `domain_id` 和 `interface` 参数通过 ROS2 参数还是环境变量传递给 unitree SDK。
3. 多 tag 同时出现在视野中时，exec_layer 如何选取当前应逼近的目标（按 `next_tag` 匹配还是最近 tag）。
4. 机器人跌倒检测通过 SportModeState 的 `error_code` 还是 IMU 数据判断。
