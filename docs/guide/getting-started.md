# 操作手册

> 控制电脑已迁移到 **Rust + ROS 2 Jazzy**。旧的 Python / ROS 2 Foxy 实现
> （`desc_layer`、`exec_layer`、`mock_exec_layer`、`planner` 等）已退役并移入
> `legacy/`，仅作参考，不再构建或运行。
>
> 所有日常任务都封装在根目录 `Makefile` 里。先看一眼：

```bash
make help
```

---

## 1. 环境要求

| 组件 | 说明 |
| --- | --- |
| Docker + Docker Compose | 控制电脑上的 ROS / Rust 开发工具链；机器人端不使用此开发容器 |
| VS Code + [Dev Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) 扩展 | 推荐；容器内自带 rust-analyzer |
| Rust 1.85（可选，host） | 只在不使用 Dev Container 时跑纯 Rust 服务测试需要 |
| Node.js 24 LTS / pnpm 9（可选，host） | 仅直接绕过 Dev Container 时需要；容器内已提供 |

`make` 会检测自己是否在容器内：`ros`、`deb`、`run-stack`、`webui`、`shell` 等目标在
host 上执行时会自动通过 `docker compose` 进入 `ros-dev:jazzy` 容器。

---

## 2. 开发环境

### 2.1 用 Dev Container（推荐）

1. 安装 VS Code 的 **Dev Containers** 扩展。
2. 打开仓库，命令面板执行 **Dev Containers: Reopen in Container**。
3. 在容器内终端构建 ROS 接口与节点：

   ```bash
   make ros
   ```

容器里已就绪：Rust 1.85 + `rust-analyzer`、ROS 2 Jazzy、colcon、`cargo-deb`。
重开容器后 rust-analyzer 会自动分析 `services/`（纯 Rust workspace）与 `ros2_ws`
下的 7 个 rclrs 节点。

### 2.2 不使用 VS Code

```bash
make image   # 首次：构建 ros-dev:jazzy 镜像
make shell   # 进入容器
```

也可以在 host 上直接调用容器型目标（会自动进入容器并复用持久卷）：

```bash
make ros
```

---

## 3. 纯 Rust 服务（host 或容器）

这些 crate 位于 `services/`，不依赖 ROS，可在 host 直接开发。

```bash
make test      # cargo test --workspace
make build     # cargo build --workspace
make fmt       # cargo fmt --all
make lint      # cargo fmt --check + cargo clippy -D warnings
make check     # lint + test
```

### 3.1 运行 gateway

```bash
make gateway   # 监听 :5000，默认 DB=/tmp/ros，maps=ros2_ws/config/maps
```

常用环境变量（见 `services/gateway/src/config.rs`）：

| 变量 | 默认 | 说明 |
| --- | --- | --- |
| `GATEWAY_HTTP_PORT` | `5000` | HTTP/WS 端口 |
| `GATEWAY_DB_DIR` | `config` | SQLite 状态目录 |
| `GATEWAY_MAPS_DIR` | `config/maps` | tag graph 目录 |
| `GATEWAY_WEBUI_DIR` | 空 | 托管 WebUI 构建产物；空则只提供 API |
| `GATEWAY_API_TOKEN` / `GATEWAY_API_TOKEN_FILE` | 空 | 设置后启用 `X-API-Key` 鉴权 |
| `GATEWAY_EXEC_ACTION` | `exec_task` | 执行 action 名 |

```bash
curl localhost:5000/api/v1/tasks
```

---

## 4. ROS 接口与节点（控制电脑）

接口用 rosidl 定义在 `ros2_ws/src/robot/interfaces/`，节点是独立的 rclrs crate
（每个节点是一个部署单元）。

```bash
make ros
```

`make ros` 会：

1. 在持久卷 `/ws` 里准备 `rosidl_rust` 与消息包；
2. 生成接口 crate 到 `/ws/install/share/<pkg>/rust`；
3. 构建 7 个节点到 `/ws/install_nodes`；
4. 写出 `ros2_ws/.cargo/config.toml`，把接口 crate 以 `[patch.crates-io]`
   指向 `/ws/...`，供容器内 rust-analyzer/cargo 解析。

> **改动任何 `.msg` / `.action` / `.srv` 后，必须重新 `make ros`**，新类型才会
> 出现在编辑器里（rosidl 代码生成的固有属性）。只改节点 `.rs` 无需重建。

首次构建较慢；之后增量。构建产物在持久 Docker 卷中，重建容器不会丢失。

### 4.1 rust-analyzer

在 Dev Container 里打开仓库即用，无需额外配置。`ros2_ws` 节点的解析依赖上面
第 4 步生成的 `ros2_ws/.cargo/config.toml`，所以请先 `make ros`。

---

## 5. 运行整栈（容器内）

```bash
make run-stack
```

`deploy/run_stack.sh` 会同时启动 `orchestrator` 与一个通用 adapter，用于控制电脑上的软件验证。可覆盖：

```bash
DEVICE_TYPE=mock make run-stack        # 默认，纯软件
DEVICE_TYPE=diff_drive make run-stack  # 内置差速仿真
DEVICE_TYPE=tonypi make run-stack      # 控制电脑 JSON-RPC 兼容路径，不等同于 RPi4B SDK exec
```

运行前需先 `make ros`（脚本会 source `/ws/install` 与 `/ws/install_nodes`）。
其他节点（`gateway_bridge`、`diagnosis_node`、`physio_mock`、`perception_*`）可按
需用 `ros2 run <pkg> <node>` 单独启动，用于调试。

### 5.1 TonyPi 实机端

TonyPi RPi4B 不运行 WebUI、gateway 或 orchestrator。它运行 Ubuntu 22.04、
ROS 2 Humble 和独立的 Python `tonypi-exec` systemd 服务。该服务加载
TonyPi Python SDK，遵循 `device_interfaces` / `task_interfaces` 的 ROS2
契约，并在本地执行 stop、watchdog 和 SDK 故障处理。控制电脑与 RPi4B
之间的 DDS/消息兼容性需要通过实机验证。

---

## 6. WebUI

WebUI 可以从 host 或 Dev Container 发起构建。host 上的 `make` 会自动进入
Dev Container；容器内已提供 Node.js 24 LTS 和 pnpm 9：

```bash
make webui       # pnpm install + vite build，产物在 webui/dist
make webui-dev   # Vite 开发服务器（默认端口 5173）
```

生产部署时把 `webui/dist` 放到 `GATEWAY_WEBUI_DIR`，由 gateway 同源托管。

---

## 7. 打包与部署

```bash
make deb         # Rust 服务（cargo-deb），产物 target/debian/*.deb
make ros-deb     # ROS 节点 + 接口，产物 dist/ros-subhealth-nodes_<ver>_amd64.deb
make publish DEBS='dist/*.deb' GPG_KEY=<key-id>   # 发布到 aptly apt 仓库
```

`make publish` 需要打包机上安装 `aptly` 与 GPG key（不在容器内）。接口包的
独立 `bloom` 发布、目标机安装、配置与密钥管理见
[`deploy/README.md`](../../deploy/README.md)。

---

## 8. 提交规范（commitlint）

提交信息通过 husky + commitlint 校验：

```bash
npm install          # 首次，安装并激活 husky hook
```

格式：`type(scope): subject`，例如：

```text
feat(adapter): enforce safety limits and watchdog
fix(gateway): return 404 for unknown task ids
docs(guide): rewrite getting-started for the Jazzy stack
```

手动校验：

```bash
npx commitlint --edit .git/COMMIT_EDITMSG
```

规则见 `commitlint.config.cjs`（scope 可选、小写；subject 允许中文；标题 ≤ 120）。

---

## 9. 常见问题

**Q: rust-analyzer 在 `ros2_ws` 里报 unresolved import（`device_interfaces` 等）？**
先生成接口 crate：在容器里执行 `make ros`。它依赖 `ros2_ws/.cargo/config.toml`
指向 `/ws/install/share/*/rust`。

**Q: `make ros` 报 `/ws` 权限错误？**
`/ws` 是持久卷，首次需归属容器用户。Dev Container 的 `postCreateCommand` 会
`chown`；手动场景可执行
`docker compose -f docker/dev/compose.yaml run --rm --user root dev chown -R ubuntu:ubuntu /ws`。

**Q: 改了接口但编辑器里类型没变？**
重新 `make ros`。

**Q: `make webui` 提示 `pnpm not found`？**
请先执行 `make image` 重建镜像，然后在 VS Code 中重连 Dev Container。
如果直接绕过容器执行 WebUI 命令，才需要在 host 安装 Node.js 24 LTS 与 pnpm 9。

**Q: `make test` 需要 ROS 吗？**
不需要。它只跑 `services/` 的纯 Rust 测试；ROS 节点在容器里构建和运行。

**Q: 运行节点时报端口/鉴权问题？**
见 §3.1 的 gateway 环境变量，以及 `deploy/config/*.env` 的默认配置。

---

## 参考

- 当前架构（权威）：[`docs/tech/tech-current-architecture.md`](../tech/tech-current-architecture.md)
- 语言边界 ADR：[`docs/tech/adr-001-language-scope.md`](../tech/adr-001-language-scope.md)
- 设备契约 ADR：[`docs/tech/adr-002-device-contract.md`](../tech/adr-002-device-contract.md)
- 设计规格：[`docs/superpowers/specs/2026-09-13-rust-ros2-jazzy-rearchitecture-design.md`](../superpowers/specs/2026-09-13-rust-ros2-jazzy-rearchitecture-design.md)
- 部署打包：[`deploy/README.md`](../../deploy/README.md)
