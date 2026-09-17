# TODO：目标架构未实现项

## 本次清理任务

- [x] 逐目录检查顶层源码、文档、部署、ROS workspace 和 WebUI。
- [x] 删除源码中的测试模块、测试目录和测试专用依赖。
- [x] 删除临时 endpoint 实现及其运行入口。
- [x] 将 `contracts/` 的接口定义迁移到 `docs/contracts/`。
- [x] 移除未实现的 ROS transport、endpoint adapter 和 ROS workspace 空壳。
- [x] 删除历史 RFC、旧实施计划、设备 spike 和旧架构图。
- [x] 更新 Make、CI、devcontainer、README 和构建文档中的失效路径。
- [x] 清理本地构建缓存和依赖缓存中的旧测试/临时产物。
- [x] 完成格式、编译、lint 和 WebUI 构建验证。

以下内容属于目标架构的一部分，但当前没有可交付实现。它们不以临时实现或空壳
目录形式留在源码中，后续实现必须遵守 `docs/architecture/` 和
`docs/contracts/`。

## P0：架构主链

- [ ] 将 Gateway 的 canonical task 接入真实 Orchestration application port。
- [ ] 将 Orchestration 接入真实 Execution port，并传播 feedback/result/cancel。
- [ ] 将 Sensor registry 接入独立 provider 生命周期，使 Orchestration 和
  Execution 共享同一数据源。
- [ ] 为 server 建立正式 composition root，启动 Gateway、Orchestration、
  Execution 和 Sensor，而不是只启动 Gateway。
- [ ] 将任务状态从当前内存实现迁移到正式持久化边界。

## P1：Transport

- [ ] 根据 `docs/contracts/` 重新建立 ROS `.msg`、`.srv`、`.action` 接口。
- [ ] 重新建立 ROS workspace、接口生成和 endpoint transport 构建入口。
- [ ] 实现 `adapters/ros-transport` 的 domain/ROS 双向 mapper。
- [ ] 实现 Gateway HTTP/WS 的完整认证、错误格式、事件重连和持久化协议。
- [ ] 让 WebUI 使用完整 canonical task/event schema，并移除历史 diagnosis
  payload 假设。

## P1：Endpoint

- [ ] 为 endpoint runtime 建立正式配置文件格式和 backend registry 加载机制。
- [ ] 实现独立 endpoint runtime，并将其与 server composition root 连接。
- [ ] 实现真实 endpoint adapter；SDK 只能位于 adapter/backend-sdk 边界。
- [ ] 实现本地 stop、watchdog、deadline、取消和故障降级。
- [ ] 为不同 Ubuntu/ROS 版本建立实际镜像 profile 与 endpoint 部署包。

## P2：平台能力

- [ ] 将地图/路径规划作为能力可选的服务接入 Orchestration。
- [ ] 恢复地图配置与地图编辑后端，并接入可选规划能力。
- [ ] 将诊断和 LLM 服务接入 Sensor/Event contract。
- [ ] 将感知数据 provider 接入 Sensor contract。
- [ ] 将安全策略接入 Execution，不把设备专属安全动作放入核心层。
- [ ] 建立 ROS、HTTP、endpoint 三类契约的版本兼容策略。

## P2：工程化

- [ ] 为正式实现建立集成验证环境；当前不保留测试代码。
- [ ] 为每个 endpoint profile 建立独立构建和部署流水线。
- [ ] 更新 CI，使其只验证格式、编译、lint、接口生成和 WebUI 构建。
- [ ] 重新审查历史 RFC/ADR，仅保留仍与当前架构相关的决策记录。
