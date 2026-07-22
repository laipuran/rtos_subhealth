# 操作手册

## 1. 环境要求

| 组件 | 版本 |
| --- | --- |
| Ubuntu | 20.04 |
| ROS2 | Foxy |
| Python | 3.8+ |
| Node.js | 18+ |
| pnpm | 9+ |
| gcc/g++ | 9.4+（编译 ROS2 接口包需要） |

### Python 依赖（desc_layer 需要）

```bash
pip install flask flask-sock
```

## 2. ROS2 环境配置

### 2.1 安装 ROS2 Foxy

参考 [ROS2 Foxy 官方安装指南](https://docs.ros.org/en/foxy/Installation/Ubuntu-Install-Debians.html)。

### 2.2 每次打开终端都需要执行

```bash
source /opt/ros/foxy/setup.bash
```

建议写入 `~/.bashrc`：

```bash
echo 'source /opt/ros/foxy/setup.bash' >> ~/.bashrc
source ~/.bashrc
```

### 2.3 DDS 环境变量

部分环境（如 WSL、Docker）不支持 DDS 多播发现，需启用 localhost-only 模式：

```bash
export ROS_LOCALHOST_ONLY=1
```

也建议写入 `~/.bashrc`：

```bash
echo 'export ROS_LOCALHOST_ONLY=1' >> ~/.bashrc
```

### 2.3 验证安装

```bash
ros2 --version
ros2 topic list   # 应无报错
```

## 3. ROS2 工作空间编译

### 3.1 首次编译

```bash
cd ros2_ws
colcon build --symlink-install
source install/setup.bash
```

`--symlink-install` 使 Python 节点可热重载，修改源码后无需重新编译。

**注意：** 编译不需要 source ROS2 以外的特殊环境变量。但运行节点前务必设置 `export ROS_LOCALHOST_ONLY=1`（见 2.3 节）。

### 3.2 只编译部分包

```bash
colcon build --packages-select exec_layer desc_layer
```

### 3.3 每次编译后

必须 source 才能发现新包或更新：

```bash
source install/setup.bash
```

### 3.4 ROS2 接口包说明

接口定义均在 `ros_interfaces` 和 `apriltag_interfaces` 中（.action / .srv / .msg）。修改接口后需重新编译这两个包：

```bash
colcon build --packages-select ros_interfaces apriltag_interfaces
source install/setup.bash
```

## 4. ROS2 包启动

### 4.1 执行层（exec_layer）

```bash
ros2 run exec_layer exec_layer_node
```

日志预期：`[INFO] [exec_layer_node]: Action server ready`

### 4.2 描述层（desc_layer）

```bash
ros2 run desc_layer desc_layer_node
```

日志预期：`[INFO] [desc_layer_node]: Desc Layer ready, connecting to action: exec_task`
内置 Flask 服务默认监听 `0.0.0.0:5000`。

**注意：** 启动前确保 `ROS_LOCALHOST_ONLY=1`（见 2.3 节），否则 DDS 多播发现可能失败。

**参数：**

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `http_port` | `5000` | HTTP/WS 服务端口 |
| `exec_action_name` | `exec_task` | 连接的执行层 action server 名称 |

连接 mock 执行层时：

```bash
export ROS_LOCALHOST_ONLY=1
ros2 run desc_layer desc_layer_node --ros-args -p exec_action_name:=mock_exec_task
```

### 4.3 感知层（AprilTag 检测）

```bash
ros2 launch apriltag_perception apriltag_perception.launch.py
```

### 4.4 相机测试发布器

```bash
ros2 launch camera_test_publisher camera_test_publisher.launch.py
```

### 4.5 Mock 执行层（mock_exec_layer，无硬件也能全链路测试）

```bash
# 安装 python 依赖（desc_layer 也需要）
pip install flask flask-sock

# 启动 mock 执行层（默认 action 名 mock_exec_task）
export ROS_LOCALHOST_ONLY=1
ros2 run mock_exec_layer mock_exec_layer_node
```

日志预期：`[INFO] [mock_exec_layer_node]: Mock Exec Layer ready on action: mock_exec_task`

**模拟行为：**

| 任务类型 | 模拟表现 |
| --- | --- |
| `go_to_tag` | 3 秒完成，每秒发一次 feedback（progress 0.3→0.6→1.0） |
| `patrol_route` | 每个 tag 耗时 1 秒，逐一发 feedback |
| `hold` | 立即返回 succeeded |
| 失败模拟 | `constraints.max_speed_mps < 0` 时返回 failed |
| 取消 | 收到 cancel 请求后立即返回 canceled |

**参数：**

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `action_name` | `mock_exec_task` | 注册的 action server 名称 |

### 4.6 查看所有可用节点

```bash
ros2 node list
ros2 topic list
ros2 action list
```

## 5. WebUI 启动

### 5.1 安装依赖

```bash
cd webui
pnpm install
```

### 5.2 开发模式启动

```bash
pnpm dev
```

默认监听 `http://localhost:5173`，Vite 自动热更新。

### 5.3 生产构建

```bash
pnpm build      # 输出到 webui/dist/
pnpm preview    # 本地预览生产构建
```

## 6. commitlint 提交规范

项目使用 husky + commitlint 自动拦截不合规的 commit message。

### 6.1 格式

```
type(scope): subject
```

**type 取值（@commitlint/config-conventional）：**

| type | 说明 |
| --- | --- |
| feat | 新功能 |
| fix | 修复 |
| docs | 文档 |
| style | 代码格式 |
| refactor | 重构 |
| test | 测试 |
| chore | 杂项（构建/CI） |
| rfc | RFC 文档专用 |

**项目内 scope 示例：** `docs`, `exec`, `desc`, `perception`, `visual`, `rfc`

### 6.2 提交示例

```
feat(docs): 更新RFC 003和RFC 006文档
fix(exec): 修复planner超时未处理的问题
rfc(docs): 添加决策层动作任务流协议
docs(guide): 添加操作手册
```

### 6.3 规则

- header 最大 120 字符
- subject 大小写不限制，允许中文
- scope 必须小写

### 6.4 绕过 commitlint（紧急情况）

```bash
git commit --no-verify -m "wip: 临时提交"
```

## 7. 全链路启动

三种模式，覆盖纯软件验证 → 仿真 → 真机。

### 7.1 Mock 模式（纯软件，推荐先验证 WebUI）

```bash
# 每个终端先 source + 设置 DDS
export ROS_LOCALHOST_ONLY=1
source /opt/ros/foxy/setup.bash
source ~/ros2_ws/install/setup.bash

# 终端 1：启动 mock 执行层
ros2 run mock_exec_layer mock_exec_layer_node

# 终端 2：启动 desc_layer（指向 mock action）
ros2 run desc_layer desc_layer_node --ros-args -p exec_action_name:=mock_exec_task

# 终端 3：启动 WebUI
cd webui && pnpm dev
```

浏览器打开 `http://localhost:5173` → 新建 go_to_tag 任务 → 实时看到 accepted → running → succeeded 状态变化。

### 7.2 仿真模式（MuJoCo，验证机器人运动）

```bash
# 终端 1：启动 MuJoCo 仿真（自动创建 DDS 桥接）
export ROS_LOCALHOST_ONLY=1
source /opt/ros/foxy/setup.bash
source ~/ros2_ws/install/setup.bash
python3 ~/unitree_mujoco/simulate_python/unitree_mujoco.py

# 终端 2：启动 exec_layer（SportClient 调用，操作仿真机器人）
export ROS_LOCALHOST_ONLY=1
source /opt/ros/foxy/setup.bash
source ~/ros2_ws/install/setup.bash
ros2 run exec_layer exec_layer_node

# 终端 3：启动 desc_layer
export ROS_LOCALHOST_ONLY=1
source /opt/ros/foxy/setup.bash
source ~/ros2_ws/install/setup.bash
ros2 run desc_layer desc_layer_node

# 终端 4：启动 WebUI
cd webui && pnpm dev
```

### 7.3 真机模式（GO2 实机）

```bash
# 终端 1：启动 exec_layer
export ROS_LOCALHOST_ONLY=1
source /opt/ros/foxy/setup.bash
source ~/ros2_ws/install/setup.bash
ros2 run exec_layer exec_layer_node

# 终端 2：启动 desc_layer
export ROS_LOCALHOST_ONLY=1
source /opt/ros/foxy/setup.bash
source ~/ros2_ws/install/setup.bash
ros2 run desc_layer desc_layer_node

# 终端 3：启动感知层
export ROS_LOCALHOST_ONLY=1
source /opt/ros/foxy/setup.bash
source ~/ros2_ws/install/setup.bash
ros2 launch apriltag_perception apriltag_perception.launch.py

# 终端 4：启动 WebUI
cd webui && pnpm dev
```

## 8. 常见问题

### Q: `colcon build` 报错找不到 package

确保先 `source /opt/ros/foxy/setup.bash`，并且 `ros2_ws/src/` 下存在对应的 package.xml。

### Q: `ros2 run` 提示找不到 package

```bash
source ros2_ws/install/setup.bash
# 验证包是否存在
ros2 pkg list | grep <包名>
```

### Q: WebUI 连接不上 desc_layer

确认 desc_layer 已启动（日志无报错），且 WebUI 的 `VITE_API_BASE` 指向正确的 desc_layer 地址（默认 `http://localhost:5000`）。

### Q: commitlint 报错

```bash
# 查看 husky hook 是否激活
ls -la .husky/commit-msg
# 手动触发校验
npx --no -- commitlint --edit .git/COMMIT_EDITMSG
```
