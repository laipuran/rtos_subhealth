# 设备无关分层系统重写设计

**状态：** 待实施

**日期：** 2026-09-17

**适用分支：** `feat.duckran.migration`

## 1. 目标

本设计将当前混合演进状态重写为一个设备无关、分层清晰、接口集中管理的
机器人任务系统。系统必须满足：

1. 控制平面不耦合任何具体机器、厂商 SDK、设备操作系统或 ROS 发行版。
2. Orchestration 只负责任务编排与生命周期，不负责设备动作实现。
3. Execution 只依赖通用执行接口，并可由不同 endpoint 实现。
4. Sensor 同时向 Orchestration 和 Execution 提供统一的数据访问能力。
5. 设备类型只存在于 endpoint 配置、endpoint adapter 和对应 backend SDK 中。
6. HTTP、WebSocket、内部服务接口和 ROS 接口均有唯一的契约来源。
7. `make` 是开发、构建和运行的统一入口。

本次采用完整重写，不以当前测试通过作为兼容目标。旧实现仅作为行为和数据
语义参考，重写后的测试必须针对新边界重新建立。

## 2. 非目标

- 不在控制平面内实现任何特定机器人动作。
- 不把某一种机器的状态机提升为全局任务模型。
- 不要求所有 endpoint 使用同一种语言、操作系统或 ROS 发行版。
- 不在本阶段实现多机器人工作流、自动建图或真实医疗设备协议。

## 3. 目标架构

```text
WebUI
  │ HTTP / WebSocket
  ▼
Gateway
  │ Task / Event Contract
  ▼
Orchestration
  ├────────────── Sensor Access ──────────────┐
  │                                           │
  ▼                                           ▼
Execution                              Sensor Providers
  │                                           ▲
  │ Execution Contract                        │
  ▼                                           │
Endpoint Runtime ─── Backend SDK ─── Device / Simulation
```

### 3.1 Gateway

Gateway 是唯一的 Web 入口，负责：

- HTTP API 和 WebSocket API；
- API 输入校验和错误格式化；
- 任务、地图和诊断结果的持久化；
- 将外部请求转换为 canonical task contract；
- 发布任务生命周期和诊断事件。

Gateway 不导入设备 SDK，不判断设备类型，不直接调用设备 endpoint。

### 3.2 Orchestration

Orchestration 负责：

- 接收 canonical task；
- 根据设备描述和能力选择可执行 endpoint；
- 进行任务排队、占用、取消、超时和生命周期管理；
- 在设备支持时调用 world model / planner；
- 订阅 Sensor 数据并据此推进编排状态。

Orchestration 不包含具体设备分支，不直接导入 ROS client 或厂商 SDK。ROS、HTTP
或其他传输由 transport adapter 负责。

### 3.3 Execution

Execution 负责：

- 把编排下发的通用 command 转为执行步骤；
- 管理执行状态、反馈、取消和 deadline；
- 订阅 Sensor 以支持行动闭环；
- 应用通用安全策略；
- 返回统一的 execution result。

Execution 不知道底层设备型号。它只依赖 `Executor`、`SensorProvider` 和
`Backend` 接口。

### 3.4 Sensor

Sensor 是独立的能力层，负责采集、缓存、发布和查询传感器数据。

```rust
pub trait SensorProvider: Send + Sync {
    fn descriptor(&self) -> SensorDescriptor;
    fn latest(&self, id: &SensorId) -> Result<SensorSample>;
    fn subscribe(&self, filter: SensorFilter) -> SensorStream;
}
```

调用关系必须同时支持：

```text
Orchestration ──读取/订阅──► Sensor
Execution ──────读取/订阅──► Sensor
```

Sensor 不决定任务，也不执行动作。具体传感器驱动属于 endpoint 或 sensor
provider adapter。

### 3.5 Endpoint Runtime 与 Backend SDK

Endpoint Runtime 是设备侧进程或节点的组合根，负责：

- 加载 endpoint 配置；
- 注册一个或多个 backend；
- 暴露统一的 execution、descriptor、state 和 sensor 接口；
- 处理本地 stop、watchdog、deadline 和故障降级。

Backend SDK 是唯一允许接触厂商协议的位置。其依赖不得反向进入 Gateway、
Orchestration、Execution 或共享 contracts。

设备类型选择必须通过配置完成：

```text
endpoint config → adapter registry → backend SDK
```

禁止在核心服务中出现 `match device_type` 后实现设备动作的分发逻辑。

## 4. 契约设计

### 4.1 Contract workspace

建立独立的契约层，至少包含：

```text
contracts/domain-contract/
contracts/task-contract/
contracts/execution-contract/
contracts/sensor-contract/
contracts/event-contract/
```

契约层只包含数据结构、枚举、错误码和 trait，不包含网络、ROS、数据库或厂商
依赖。

### 4.2 Task contract

Canonical task 至少包含：

- `task_id`；
- 目标 endpoint 或能力约束；
- primitive / action；
- target；
- 参数；
- deadline；
- cancellation policy。

通用 primitive 只表达跨设备语义，例如 `hold`、`stop`、`execute_primitive`。
只有具备明确闭环能力的 endpoint 才声明 `move_to_pose` 或 `set_velocity`。

### 4.3 Device contract

设备描述只声明事实：

- `device_id`；
- 能力集合；
- 支持的 primitive；
- 限制；
- 坐标系；
- 可用 sensors；
- 当前健康状态。

核心代码按照能力匹配设备，不按照具体设备名称或厂商名称分支。

### 4.4 Execution contract

```rust
pub trait Executor: Send + Sync {
    fn descriptor(&self) -> DeviceDescriptor;
    fn execute(&self, command: ExecutionCommand) -> Result<ExecutionHandle>;
    fn cancel(&self, task_id: &TaskId) -> Result<()>;
    fn state(&self) -> DeviceState;
}
```

执行结果统一包含：状态、进度、阶段、错误码、错误信息和时间戳。

### 4.5 Transport contract

Transport 只负责序列化和传输，不重新定义业务语义：

- HTTP/WS 映射 `gateway-contract`；
- ROS 2 映射 `task-contract`、`execution-contract`、`sensor-contract`；
- endpoint 本地协议映射 `execution-contract` 和 `sensor-contract`。

ROS `.msg`、`.srv`、`.action` 与 Rust domain 类型之间必须有显式 mapper，禁止
业务核心直接使用生成的 ROS 类型。

## 5. 代码仓库目标结构

```text
contracts/
  domain-contract/
  task-contract/
  execution-contract/
  sensor-contract/
  event-contract/

services/
  gateway/
  orchestration/
  execution/
  sensor/
  world-model/
  diagnosis/
  safety/

adapters/
  gateway-http/
  gateway-ws/
  ros-transport/
  endpoint-runtime/
  endpoint-adapters/
  backend-sdks/

interfaces/ros/
webui/
deploy/
docs/
```

允许的依赖方向：

```text
transport / adapter → services → contracts
backend SDK → endpoint adapter → endpoint runtime → contracts
```

禁止反向依赖和跨层调用。特别是：

- `contracts` 不依赖任何 service；
- `orchestration` 不依赖 endpoint adapter；
- `execution` 不依赖具体 SDK；
- `gateway` 不依赖设备实现；
- ROS generated types 不进入 domain/service 核心。

## 6. Make 与运行模型

所有 `.PHONY` 目标在根 `Makefile` 中定义。

```bash
make webui
make webui-dev

make run server
make run endpoint DEVICE_TYPE=<device-type>

make image humble
make image jazzy
```

`server` 和 `endpoint` 是 `run` 的两种互斥模式。

镜像选择由 Ubuntu 版本、ROS 版本、endpoint 类型和 endpoint backend SDK 共同
决定。设备类型只作为 endpoint 运行配置传入，不参与 server 或 orchestration
核心编译。

Make 必须拒绝缺少 endpoint device type 的命令，并对不支持的组合给出明确错误。

## 7. 重写策略

重写按以下顺序进行：

1. 删除旧 workspace 成员、旧 ROS 节点入口和重复的设备分发包。
2. 创建 contracts，并为每个契约建立独立单元测试和序列化 fixture。
3. 重写 Sensor service 与 provider interface。
4. 重写 Execution runtime，先使用 fake backend 和 fake sensor。
5. 重写 Orchestration，使用 execution contract 与 sensor contract。
6. 重写 Gateway transport 和持久化边界。
7. 重写 ROS transport 与 endpoint runtime。
8. 将具体 backend SDK 迁移到 endpoint adapter 目录。
9. 重写 WebUI API client、Make、Docker 和部署配置。
10. 将旧文档标记为 superseded，并以本架构文档和接口文档为唯一入口。

重写期间不保留旧实现的兼容分支；需要保留的外部 API 语义在新的 adapter 中
重新实现，而不是复用旧模块内部结构。

## 8. 测试策略

测试围绕新边界重新建立：

| 层 | 测试内容 |
|---|---|
| contracts | 序列化、反序列化、错误码、版本兼容 |
| Sensor | fake provider、缓存、订阅、丢失和过期数据 |
| Execution | fake backend、fake sensor、取消、deadline、安全停止 |
| Orchestration | 能力匹配、生命周期、占用、重试、失败释放 |
| Gateway | HTTP/WS contract、持久化、事件顺序和鉴权 |
| transport | domain 与 ROS/HTTP payload 双向映射 |
| endpoint | adapter/backend SDK 隔离和本地安全行为 |
| integration | fake endpoint 完成 WebUI 到执行结果的全链路 |

任何测试不得要求真实设备才能运行。真实设备测试只验证 endpoint adapter 和
backend SDK，不作为核心服务测试前置条件。

## 9. 验收标准

完成重写后必须满足：

1. `cargo tree` 中核心 services 不出现任何具体设备 SDK。
2. 全仓库核心代码不存在按设备名称分发动作的条件分支。
3. Sensor 可被 Orchestration 和 Execution 同时访问。
4. 更换 endpoint backend 时不修改 Gateway、Orchestration 或 Execution。
5. 新增 endpoint 只需新增 adapter、backend SDK 组合和配置。
6. `make run server` 不需要任何设备类型。
7. `make run endpoint DEVICE_TYPE=x` 能明确选择 endpoint 类型。
8. `make image humble` 与 `make image jazzy` 选择不同 ROS 镜像配置。
9. 接口文档只有一个权威入口，旧 RFC 明确标记为 superseded 或 archive。
10. fake endpoint 可以完成任务提交、Sensor 支持、执行反馈、取消和最终结果。
