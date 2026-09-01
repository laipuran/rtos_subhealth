# RFC 009 测试方案

> 依据 [RFC 009: 生理传感数据驱动的 LLM 健康诊断层](rfc-009-llm-health-diagnosis.md) 制定，
> 覆盖单元测试、集成测试、API 测试与并发/边界测试四大维度。

---

## 1. 测试环境与前置条件

### 1.1 环境要求

| 项 | 说明 |
|---|---|
| Python | ≥ 3.10 |
| pytest | ≥ 7.0（`requirements.txt` 已声明） |
| ROS2 | Humble/Iron（集成测试需要） |
| 依赖包 | `requests`, `numpy`, `pytest` |

### 1.2 目录结构

```
diagnosis_layer/
├── diagnosis_layer/
│   ├── aggregator.py          # 窗口聚合 + 异常检测
│   ├── config.py              # 配置加载
│   ├── llm_client.py          # LLM 调用 + JSON 校验
│   ├── diagnosis_layer_node.py # ROS2 节点（编排）
│   ├── demo.py                # 免 ROS 演示
│   ├── tests/
│   │   ├── test_aggregator.py
│   │   ├── test_config.py
│   │   ├── test_llm.py
│   │   ├── test_rag.py
│   │   └── test_diagnosis_store.py
│   └── rag/
│       ├── corpus.py
│       ├── embeddings.py
│       └── retriever.py
```

### 1.3 运行命令

```bash
# 单元测试（无需 ROS2 环境）
cd ros2_ws/src/orchestration/diagnosis_layer
python -m pytest tests/ -v

# 集成测试（需要 ROS2 环境）
cd ros2_ws && ./run.sh mock
```

---

## 2. 单元测试

### 2.1 聚合器测试（`test_aggregator.py`）

#### 2.1.1 窗口统计

| 用例 | 输入 | 期望输出 |
|---|---|---|
| 正常统计 | `Window("mock_spo2","spo2")` 添加 `[88,89,87,86]` | `mean=87.5, min=86, max=89, valid=True` |
| 空窗口 | 新建 Window，不添加样本 | `mean=None, valid_count=0, valid=False` |
| 全部无效 | 添加 `valid=False` 的样本 | `valid=False` |
| 趋势检测 | 4+ 个样本且递增 | `trend="increasing"` |
| 趋势检测 | 4+ 个样本且递减 | `trend="decreasing"` |
| 趋势检测 | < 4 个样本 | `trend="unknown"` |
| 趋势检测 | 变化量 < 0.5 | `trend="stable"` |
| 四舍五入 | 均值 87.556 | `mean=87.56` |

#### 2.1.2 窗口修剪

| 用例 | 输入 | 期望输出 |
|---|---|---|
| 修剪过期样本 | window=10s, 样本 t=0,5,20, now=25 | 仅保留 t=20 的样本 |
| 全部过期 | 所有样本 t < now - window | 空列表 |
| 无过期 | 所有样本在窗口内 | 全部保留 |
| 乱序时间戳 | 按 t=10,2,8 顺序添加后 prune | 正确按 t 排序修剪 |

#### 2.1.3 异常检测

| 用例 | data_type | value | thresholds | 期望 |
|---|---|---|---|---|
| spo2 低值 | spo2 | 85 | 默认 | `anomalous=True` |
| spo2 正常值 | spo2 | 97 | 默认 | `anomalous=False` |
| heart_rate 高值 | heart_rate | 110 | 默认 | `anomalous=True` |
| heart_rate 低值 | heart_rate | 55 | 默认 | `anomalous=True` |
| systolic 高值 | systolic_mmhg | 145 | 默认 | `anomalous=True` |
| diastolic 高值 | diastolic_mmhg | 95 | 默认 | `anomalous=True` |
| body_temp 高值 | body_temp_c | 38 | 默认 | `anomalous=True` |
| body_temp 低值 | body_temp_c | 34 | 默认 | `anomalous=True` |
| respiratory_rate 高值 | respiratory_rate | 22 | 默认 | `anomalous=True` |
| respiratory_rate 正常 | respiratory_rate | 16 | 默认 | `anomalous=False` |
| 未知类型 | unknown_type | 100 | 默认 | `anomalous=False` |
| 自定义阈值 | spo2 | 91 | `{"low":92}` | `anomalous=True` |
| 空阈值 | spo2 | 80 | `{}` | `anomalous=False` |

#### 2.1.4 快照构建

| 用例 | 输入 | 期望输出 |
|---|---|---|
| 单源快照 | windows={"mock_spo2": win}, trigger="periodic" | `trigger_type="periodic"`, `sources` 含 1 项 |
| 多源快照 | windows={"mock_spo2": w1, "mock_heart_rate": w2} | `sources` 含 2 项, types 正确 |
| 字段完整性 | 任意窗口 | 每个 source 含 data_src, data_type, mean, min, max, latest, trend, valid |

#### 2.1.5 阈值解析

| 用例 | 输入 | 期望 |
|---|---|---|
| 空字符串 | `""` | 返回默认 `ANOMALY_THRESHOLDS` |
| 合法 JSON | `'{"spo2":{"low":92}}'` | 自定义生效 |
| 非法 JSON | `"not json"` | 返回默认值 |
| 非 dict JSON | `"[]"` | 返回默认值 |
| 部分字段 | `'{"spo2":{"low":92}}'` | 缺失的 high 为 None |

---

### 2.2 配置测试（`test_config.py`）

| 用例 | 输入 | 期望 |
|---|---|---|
| 无输入 | `build_config({}, "")` | 使用全部 DEFAULTS |
| 环境变量覆盖 | `LLM_API_KEY=sk-test` | `cfg["llm_api_key"]=="sk-test"` |
| OpenAI 速记 | `OPENAI_API_KEY=sk-test` | `llm_base_url="https://api.openai.com"` |
| ROS 参数优先 | ROS `llm_api_key=param` + env `env-key` | `param-key` |
| 配置文件加载 | 临时 JSON 文件 | 文件值生效 |
| 优先级链 | DEFAULTS < 文件 < ROS 参数 | 逐层覆盖正确 |

---

### 2.3 LLM 客户端测试（`test_llm.py`）

#### 2.3.1 JSON 解析

| 用例 | 输入 | 期望 |
|---|---|---|
| 合法 JSON | `{"severity":"mild",...}` | 正确解析 |
| JSON 围栏 | `\`\`\`json\n{...}\n\`\`\`` | 提取内部 JSON |
| 非法 severity | `{"severity":"weird",...}` | `ValueError` |
| 缺字段 | `{"severity":"normal"}` | `ValueError` |
| confidence 非数字 | `{"confidence":"abc",...}` | `ValueError` |
| confidence 越界 | `{"confidence":1.5,...}` | `ValueError` |
| confidence 边界 | `{"confidence":0.8}` | 通过 `passes_confidence` |
| possible_cases 非数组 | `{"possible_causes":"str",...}` | 自动转为 `["str"]` |

#### 2.3.2 置信度筛选

| 用例 | obj | confidence_min | 期望 |
|---|---|---|---|
| 低于阈值 | `{"confidence":0.7}` | 0.8 | `False` |
| 等于阈值 | `{"confidence":0.8}` | 0.8 | `True` |
| 高于阈值 | `{"confidence":0.85}` | 0.8 | `True` |
| 缺失 confidence | `{}` | 0.8 | `False` |

#### 2.3.3 Prompt 构建

| 用例 | 输入 | 期望 |
|---|---|---|
| 注入上下文 | snapshot + context | context 出现在 user 中 |
| 无上下文 | snapshot + `""` | "（无可用资料）" |
| 字段完整 | 含 severity, summary 等 | system 含所有必填字段声明 |

#### 2.3.4 LLM 禁用

| 用例 | 输入 | 期望 |
|---|---|---|
| 无 base_url | `LLMClient()` | `enabled=False` |

---

### 2.4 RAG 检索测试（`test_rag.py`）

#### 2.4.1 语料加载

| 用例 | 输入 | 期望 |
|---|---|---|
| 正常加载 | 含 .md 文件的目录 | Chunk 列表，非空 |
| 空目录 | 空目录 | 空列表 |
| 目录不存在 | 不存在路径 | 空列表 |
| 章节提取 | 含 `#` 和 `##` 的 Markdown | 按章节分块正确 |

#### 2.4.2 关键词检索

| 用例 | 输入 | 期望 |
|---|---|---|
| 关键词匹配 | spo2 异常快照 | 检索到含"血氧"的 chunk |
| top_k 限制 | top_k=1 | 返回 ≤ 1 条 |
| 空语料 | 空目录 | 返回空列表 |
| 嵌入模式 | EmbeddingClient 可用 | mode="embeddings" |
| 嵌入回退 | Embedding 失败 | 自动回退 keyword |

#### 2.4.3 查询构建

| 用例 | 输入 | 期望 |
|---|---|---|
| 含 data_type | sources 含 spo2 | query 含"spo2" |
| 含 trigger_type | trigger_type="anomaly" | query 含"anomaly" |
| 传感器异常 | valid=false | query 含"sensor abnormal" |

#### 2.4.4 上下文格式化

| 用例 | 输入 | 期望 |
|---|---|---|
| 多个 chunk | ["a","b"] | `format_context` 以 `\n\n` 连接 |
| 超长截断 | 总字符 > 4000 | 截断至 4000 字符内 |

---

### 2.5 诊断存储测试（`test_diagnosis_store.py`）

| 用例 | 输入 | 期望 |
|---|---|---|
| 持久化添加 | 添加记录 | 可通过 ID 查询 |
| 过期清理 | purge_older_than(3600) | 旧记录被删除，返回计数 |
| 零值不清理 | purge_older_than(0) | 无记录被删除 |
| 列表分页 | list_all(offset, limit) | 正确分页 |
| 计数 | count() | 正确计数 |

---

## 3. 集成测试（`./run.sh mock`）

### 3.1 全链路冒烟测试

**场景：** `./run.sh mock` 启动后，mock 发布器以 1 Hz 发布 6 个传感器数据

| 步骤 | 操作 | 期望 |
|---|---|---|
| 1 | 启动 `./run.sh mock` | 节点初始化日志包含 `Diagnosis layer ready` |
| 2 | 等待一个周期（默认 60s） | `/diagnosis/results` 收到 DiagnosisResult |
| 3 | 检查 WS 事件 | 收到 `event: "diagnosis"` 载荷 |
| 4 | 检查 desc_layer | SQLite 中存在新记录 |
| 5 | 检查 HTTP API | `GET /api/v1/diagnostics` 返回该记录 |

### 3.2 异常自动触发

**场景：** mock 以 `scenario=anomaly` 发布 spo2 ≈ 85（低于阈值 90）

| 步骤 | 操作 | 期望 |
|---|---|---|
| 1 | 启动 `./run.sh mock` 并设置 `scenario=anomaly` | mock 发布 spo2 ≈ 85 |
| 2 | 等待异常检测 | anomaly 状态转换触发诊断任务 |
| 3 | 检查结果 | DiagnosisResult 的 `trigger_type="anomaly"` |
| 4 | 检查 severity | `severity` ∈ {mild, moderate, severe, critical} |
| 5 | 检查 source_ids | 包含 `"mock_spo2"` |

### 3.3 手动触发

| 步骤 | 操作 | 期望 |
|---|---|---|
| 1 | 调用 `POST /api/v1/diagnostics` | 返回 `202`，`status: "triggered"` |
| 2 | 检查 `/diagnosis/trigger` topic | 收到 `String.data="manual:<trace_id>"` |
| 3 | 等待诊断完成 | WS 收到 `diagnosis` 事件，`trace_id` 匹配 |

### 3.4 LLM 不可用降级

**场景：** 不配置 LLM 环境变量

| 步骤 | 操作 | 期望 |
|---|---|---|
| 1 | 启动无 LLM 配置 | 节点日志显示 `LLM enabled=False` |
| 2 | 等待诊断完成 | 发布 `error_code="LLM_DISABLED"` 的 DiagnosisResult |
| 3 | 检查 summary | 为 RAG 上下文前 500 字符 |

### 3.5 定时周期触发

| 步骤 | 操作 | 期望 |
|---|---|---|
| 1 | 启动 `./run.sh mock` | periodic 定时器启动 |
| 2 | 等待 `periodic_interval_s`（默认 60s） | 收到 `trigger_type="periodic"` 的诊断 |
| 3 | 再次等待一个周期 | 再次收到诊断结果 |

---

## 4. 并发与优先级测试

### 4.1 相同任务去重

| 用例 | 场景 | 期望 |
|---|---|---|
| 重复 periodic | 上一个 periodic 任务尚未完成 | 新任务被忽略（`skip duplicate job`） |
| 重复 anomaly | 同一传感器异常冷却期内 | 不重复触发（`anomaly_cooldown_s=30s`） |

### 4.2 异常优先级

| 用例 | 场景 | 期望 |
|---|---|---|
| 异常抢占 | periodic 任务运行中，异常触发 | anomaly 任务可降级 periodic 的 RAG 检索（`anomaly_busy` 跳过可选检索） |
| 多异常并发 | 多个传感器同时异常 | 每个传感器独立触发，不互相阻塞 |
| 异常恢复 | 异常值回到正常范围 | `anomaly_state` 置 False，下次异常重新触发（恢复后不重复触发） |

### 4.3 并发安全

| 用例 | 场景 | 期望 |
|---|---|---|
| 多订阅回调 | 6 个传感器同时发布 | `_on_sample` 线程安全（`_lock` 保护） |
| 定时器 + 消息 | periodic 定时器与消息到达同时 | 无竞态条件 |

---

## 5. API 测试

### 5.1 手动触发诊断

| 用例 | 请求 | 期望 |
|---|---|---|
| 正常触发 | `POST /api/v1/diagnostics`（无 token） | `202`，`body.status: "triggered"` |
| 带鉴权 | `POST /api/v1/diagnostics` + `X-API-Key` | `X-Trace-Id` 头存在 |
| 无效 JSON | 无 body | 错误处理 |

### 5.2 诊断记录列表

| 用例 | 请求 | 期望 |
|---|---|---|
| 分页 | `GET /api/v1/diagnostics?offset=0&limit=10` | 返回最多 10 条记录 |
| 统一错误格式 | 不存在的 ID 格式 | `{"error": {"code": ..., "message": ...}}` |
| Trace 贯穿 | 所有请求 | 响应头含 `X-Trace-Id` |

### 5.3 单条诊断详情

| 用例 | 请求 | 期望 |
|---|---|---|
| 存在记录 | `GET /api/v1/diagnostics/{id}` | 返回完整 DiagnosisRecord |
| 不存在记录 | `GET /api/v1/diagnostics/nonexistent` | `404`，`error.code: "NOT_FOUND"` |

### 5.4 鉴权

| 用例 | 请求 | 期望 |
|---|---|---|
| 无 token（有 api_token） | `GET /api/v1/diagnostics` | `401`，`error.code: "UNAUTHORIZED"` |
| 错误 token | `X-API-Key: wrong` | `401` |
| 无 api_token 配置 | 任意请求 | 跳过鉴权 |

---

## 6. 边界与异常测试

### 6.1 时间边界

| 用例 | 场景 | 期望 |
|---|---|---|
| 乱序时间戳 | 样本按 t=10, 2, 8 到达 | 窗口统计基于有效时间窗口 |
| 大时间跳跃 | 样本 t 远大于当前 | 旧样本被正确修剪 |
| future 时间戳 | 样本时间在未来 | 不影响当前窗口统计 |

### 6.2 数据边界

| 用例 | 场景 | 期望 |
|---|---|---|
| `valid=false` | 传感器异常样本 | 标记为传感器异常，触发 anomaly |
| 空窗口统计 | 无有效样本 | `valid=False, mean=None` |
| 单个样本 | 窗口仅 1 个样本 | `trend="unknown"` |
| 极端值 | value 极大/极小 | 不崩溃，正确计算 |

### 6.3 LLM 输出边界

| 用例 | 场景 | 期望 |
|---|---|---|
| JSON 围栏 | LLM 输出含 \`\`\`json 围栏 | 正确提取 JSON |
| 裸 JSON 块 | LLM 输出含 `{...}` | 正则提取 |
| 无 JSON | LLM 输出纯文本 | `LLM_PARSE_FAILED`，重试 3 次 |
| 3 次重试失败 | 连续 3 次解析失败 | 发布 `error_code="LLM_PARSE_FAILED"` |
| confidence 低于阈值 | confidence=0.7, threshold=0.8 | `error_code="LOW_CONFIDENCE"` |

### 6.4 RAG 边界

| 用例 | 场景 | 期望 |
|---|---|---|
| 无医学资料 | `docs/medical/` 为空 | 无 RAG 上下文，LLM 仍可生成 |
| Embedding 不可用 | embedding 端点报错 | 自动回退关键词检索 |
| 空查询 | snapshot 无 sources | `build_query` 不崩溃 |

---

## 7. 消息与接口合规测试

### 7.1 PhysioSample 消息

| 字段 | 类型 | 合规检查 |
|---|---|---|
| `timestamp` | `builtin_interfaces/Time` | ROS2 时间戳 |
| `data_src` | `string` | 如 `device_spo2`, `mock_heart_rate` |
| `data_type` | `string` | ∈ {spo2, heart_rate, systolic_mmhg, diastolic_mmhg, body_temp_c, respiratory_rate} |
| `data` | `float32` | 单位随 data_type |
| `valid` | `bool` | false=传感器异常/缺数 |

### 7.2 DiagnosisResult 消息

| 字段 | 类型 | 合规检查 |
|---|---|---|
| `diagnosis_id` | `string` | UUID hex[:16] 或手动指定 |
| `source_ids` | `string[]` | 参与诊断的 data_src 集合 |
| `trigger_type` | `string` | ∈ {periodic, anomaly, manual} |
| `severity` | `string` | ∈ {normal, mild, moderate, severe, critical} |
| `confidence` | `float32` | [0, 1]，低于 `confidence_min` 被筛除 |
| `raw_prompt` | `string` | 完整的 system+user prompt |
| `error_code` | `string` | LLM 失败时有值 |
| `error_message` | `string` | 失败原因描述 |

### 7.3 WebSocket 事件

| 字段 | 合规检查 |
|---|---|
| `event` | `"diagnosis"` |
| `trace_id` | 与 `diagnosis_id` 相同（RFC-009 §5.4） |
| 载荷字段 | diagnosis_id, trigger_type, severity, summary, possible_causes, recommendations, confidence, timestamp |

---

## 8. 测试执行矩阵

| 测试类别 | 用例数 | 执行方式 | 依赖 ROS2 | 执行频率 |
|---|---|---|---|---|
| 单元测试-聚合器 | 10+ | `pytest tests/test_aggregator.py` | 否 | 每次提交 |
| 单元测试-配置 | 5+ | `pytest tests/test_config.py` | 否 | 每次提交 |
| 单元测试-LLM | 6+ | `pytest tests/test_llm.py` | 否 | 每次提交 |
| 单元测试-RAG | 6+ | `pytest tests/test_rag.py` | 否 | 每次提交 |
| 单元测试-存储 | 2+ | `pytest tests/test_diagnosis_store.py` | 否 | 每次提交 |
| 集成测试-mock | 5+ | `./run.sh mock` + ROS2 CLI | 是 | 每日/PR |
| 集成测试-并发 | 3+ | 脚本模拟并发 | 是 | 每日/PR |
| API 测试 | 8+ | `curl`/`requests` + Flask test client | 部分 | 每次提交 |
| 边界测试 | 15+ | 单元测试 + 集成测试 | 部分 | 每次提交 |

---

## 9. 通过标准

### 9.1 单元测试

- 所有测试用例 **100% 通过**
- `pytest --tb=short -v` 无失败、无错误

### 9.2 集成测试

- `./run.sh mock` 下全链路跑通（mock vitals → 窗口聚合 → 异常触发 → WS 收到 diagnosis）
- 手动触发 API 返回 `202`，WS 收到对应事件

### 9.3 API 测试

- 鉴权、分页、统一错误格式、`trace_id` 贯穿均正确

### 9.4 并发测试

- 定时任务运行中触发异常任务不被阻塞
- 异常触发拥有最高优先级

---

## 10. 测试覆盖率目标

| 模块 | 目标覆盖率 |
|---|---|
| `aggregator.py` | ≥ 90%（所有 public 函数） |
| `config.py` | ≥ 95%（所有分支） |
| `llm_client.py` | ≥ 90%（parse_diagnosis, passes_confidence, build_messages） |
| `rag/retriever.py` | ≥ 85%（keyword + embedding 路径） |
| `rag/corpus.py` | ≥ 90%（load_corpus, _chunk_file） |
| `diagnosis_layer_node.py` | ≥ 70%（核心路径，ROS2 节点集成） |
| `diagnosis_store.py` | ≥ 85%（add, get, list, purge） |
| `http_server.py` | ≥ 80%（diagnostics 路由） |