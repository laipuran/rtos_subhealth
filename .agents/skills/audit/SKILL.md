---
name: codebase-industrialization-audit
description: Audit a codebase for contract completeness, code quality, implementation correctness, log quality, parameter injection, hardcoded issues, architecture decoupling, and industrialization readiness. Applicable to any language/framework.
---

# Codebase Industrialization & Quality Audit

## 1. 契约完整性 (Contract Completeness)

- [ ] **接口定义 vs 实现一致性**: 接口定义文件（Proto / OpenAPI / `.action` / `.srv` / `.msg` / Type 定义 / Interface）与实际消费/实现方是否一致，字段增减和类型变更是否在所有相关方同步更新
- [ ] **请求/响应结构**: 同一功能的请求与响应结构是否在上下游间保持对称，无冗余字段或缺失字段
- [ ] **错误响应标准化**: 所有 API / RPC 的错误响应是否遵循统一格式（如 `{"error": {"code": "...", "message": "..."}, "trace_id": "..."}`），错误码是否有枚举定义且语义清晰
- [ ] **未实现扫描**: 搜索 `TODO` / `FIXME` / `HACK` / `XXX` / `NotImplementedError` / `raise NotImplementedError` / `pass` 空函数体，统计未实现占比并评估风险

## 2. 代码质量 (Code Quality)

- [ ] **静态类型**: 是否启用语言级类型检查（Python type hints / TypeScript `strict` / C++ 强类型），类型覆盖率如何，有无过度使用 `Any` / `any` / `void*`
- [ ] **异常/错误处理**: 是否存在裸 `except:`、空的 `except Exception: pass`、未处理的 `Optional`/`null` 路径、未检查的返回值；是否在合适抽象层捕获并转换错误
- [ ] **重复代码**: 相同逻辑或工具函数是否在多个模块间复制粘贴，能否抽取公共库
- [ ] **函数复杂度**: 函数行数是否过长、嵌套深度是否超标（建议不超过 4 层）、是否违反单一职责原则
- [ ] **命名一致性**: 同层抽象是否遵循统一命名风格（变量/函数/类/模块/接口），命名能否自文档化

## 3. 实现正确性 (Implementation Correctness)

- [ ] **并发安全**: 共享状态（全局变量、缓存、队列）是否有锁 / 原子操作 / 队列保护；回调函数是否线程安全；异步任务有无竞态条件
- [ ] **状态机/生命周期**: 状态转换图是否完备，非法转换是否被显式拦截；资源初始化与销毁是否配对
- [ ] **边界条件**: 空输入、零值、极值、超时、网络断连、资源耗尽（磁盘/OOM/FD 泄漏）时的行为是否符合预期
- [ ] **资源管理**: 连接池、文件句柄、订阅者/发布者、进程/线程是否在生命周期结束时正确释放（`close` / `__exit__` / `dispose` / RAII）
- [ ] **幂等性**: 同一请求重复执行（重试、重复提交）是否产生副作用，关键操作是否有去重/防重入机制

## 4. 日志合理性 (Log Quality)

- [ ] **日志级别**: 是否合理区分 `DEBUG` / `INFO` / `WARN` / `ERROR`，无信息倒挂（如关键错误打 `INFO`、调试信息打 `ERROR`），生产环境日志级别是否可控
- [ ] **关键上下文**: 请求入口、状态变更、决策分支、错误发生时是否记录了足够上下文（请求 ID / trace ID / 业务主键 / 决策原因），便于问题定位
- [ ] **日志频率**: 高频路径（循环、回调、轮询）中日志是否可控，有无降频/采样/节流机制，防水日志打爆磁盘
- [ ] **敏感信息**: 日志中是否泄露 IP 地址、端口、Token、密钥、证书、内部文件路径、个人身份信息
- [ ] **结构化/可检索**: 日志格式是否统一（时间戳、级别、模块、消息、结构化字段），关键字段能否通过 grep 快速过滤

## 5. 参数注入与硬编码 (Parameter Injection & Hardcoded Issues)

- [ ] **环境差异配置**: 不同环境（开发/测试/生产）的差异配置（数据库、地址、端口、日志级别）是否通过参数 / 环境变量 / 配置中心注入，而非硬编码在代码中
- [ ] **业务常量**: 超时时间、限值阈值、重试次数、地址、路径等业务常量是否集中管理（常量文件 / 配置对象 / 枚举），而非散布在代码各处
- [ ] **凭据管理**: 密码、API Key、Token、证书等敏感凭据是否从环境变量或密钥管理服务读取，严禁代码内硬编码
- [ ] **配置加载**: 配置文件的加载顺序、覆写机制、格式校验是否清晰；缺少配置时是否能给出明确提示而非静默使用默认值

## 6. 架构解耦与依赖方向 (Architecture Decoupling)

- [ ] **依赖方向**: 模块间依赖是否遵循单向规则（如 `UI → Service → Data`），检查是否存在循环依赖或反向依赖
- [ ] **抽象稳定性**: 接口/抽象基类是否稳定，具体实现是否可替换；高层模块是否依赖抽象而非具体实现（DIP）
- [ ] **全局/单例状态**: 全局可变状态、模块级单例是否可控；有无隐式共享状态导致测试相互影响或并发问题
- [ ] **跨层通信**: 不同抽象层之间的通信是否遵守分层约定（如 UI 不直连数据库、前端不直连中间件），通信协议选择是否合理（同步 vs 异步、RPC vs 消息队列）

## 7. 工业化程度 (Industrialization Readiness)

- [ ] **CI/CD 自动化**: 是否配置 Lint → Test → Build → Deploy 流水线（如 GitHub Actions / GitLab CI / Jenkins）；提交是否通过门禁检查
- [ ] **测试覆盖**: 是否覆盖 UT（单元测试） / IT（集成测试） / E2E（端到端测试）；核心业务逻辑和边界场景是否有测试用例；测试是否与生产代码一并维护
- [ ] **代码规范工具**: 是否配置 Linter（ESLint / Ruff / flake8）+ Formatter（Prettier / Black）+ Type Checker（TypeScript / mypy / pyright），且与 CI 集成
- [ ] **贡献规范**: 是否有 CONTRIBUTING.md 或类似文档，约定提交信息格式、分支策略、Code Review 流程
- [ ] **容器化/部署**: 是否有 Dockerfile / docker-compose / Helm Chart / 部署脚本，环境一键部署能力如何
