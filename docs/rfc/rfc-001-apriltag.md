## RFC 001: AprilTag 感知发布协议

**状态：** 草案

**修订日期：** 2026-03-29

## 1. 摘要
本 RFC 定义 AprilTag 检测结果的发布规范，用于医疗场景的 tag chaining，而非 Nav2 式二维导航。协议统一输出 topic 与消息字段，便于后续规划/执行层使用。影响范围仅限感知发布接口，不涉及相机发布与底层控制。

---

## 2. 目标与非目标
**目标：**
1. 为 tag chaining 提供稳定、可复用的检测输出。
2. 统一字段含义、单位与取值范围，降低下游解析成本。
3. 无目标时持续发布空数组，保证链路稳定性。

**非目标：**
1. 不定义相机 topic 的发布方式与标定流程。
2. 不负责识别后对底层硬件的操控。
3. 不解决二维导航/建图与全局路径规划问题。

---

## 3. 现状与痛点
医疗场景不需要 Nav2 的二维导航，但仍需要稳定的 tag chaining 输入。没有统一发布协议时，下游对字段、单位和频率理解不一致，影响链路对齐。

---

## 4. 方案概览
通过固定的 AprilTag 检测输出接口，提供可直接使用的 tag chaining 信息，降低上下游歧义。
数据流：camera topic -> AprilTag 检测节点 -> `/perception/apriltag_detections`。

---

## 5. 关键接口
**输出 topic：** `/perception/apriltag_detections`

**消息类型：**
- `apriltag_interfaces/AprilTagDetection`
- `apriltag_interfaces/AprilTagDetections`

**检测项（单目标）：**
| 字段 | 类型 | 单位 | 说明 |
| :--- | :--- | :--- | :--- |
| `id` | int32 | - | AprilTag 编号，映射医疗点位 |
| `distance` | float32 | mm | 相机中心到 Tag 中心直线距离 |
| `center_offset_x` | float32 | 归一化 | 水平偏差，范围 `[-1.0, 1.0]` |
| `center_offset_y` | float32 | 归一化 | 垂直偏差，范围 `[-1.0, 1.0]` |
| `roll` | float32 | deg | 翻滚角 |
| `yaw` | float32 | deg | 偏航角 |
| `pitch` | float32 | deg | 俯仰角 |
| `hamming` | int32 | - | 汉明距离，`0` 为完美匹配 |

**检测数组（一帧）：**
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `timestamp` | builtin_interfaces/Time | ROS 时间戳 |
| `frame_id` | string | 相机坐标系（默认 `camera_link`，可参数化） |
| `detections` | 检测项数组 | 当前帧所有目标；无目标时为空数组 |

**发布要求：** 发布频率 `>= 10 Hz`；空集持续发布；`distance` 为 mm；姿态角为 deg。

---

## 6. 数据/控制流
1. 感知节点从 camera topic 读取图像并检测 AprilTag。
2. 检测结果发布到 `/perception/apriltag_detections`。
3. 下游模块（规划/执行/控制）按统一字段含义使用。

---

## 7. 风险与替代
1. **替代方案：** 采用 Nav2/二维导航栈。
   - **权衡：** 能提供全局规划与避障，但引入重依赖与高算力开销，不符合 tag chaining 轻量场景。
2. **替代方案：** 使用已有标准消息（如 apriltag_ros/vision_msgs 系列）。
   - **权衡：** 兼容外部生态，但字段含义与本场景不完全匹配，仍需二次转换。
3. **风险：** 自定义消息与 topic 固化后，跨团队协作更依赖此 RFC 的一致性维护。

---

## 8. 验证计划
1. topic 发布频率稳定不低于 10Hz。
2. 无目标场景持续发布空数组。
3. 字段单位与取值范围与 RFC 一致。
4. 异常输入下节点不崩溃，持续发布。

---

## 9. 未决问题
1. 输出 topic 的 QoS 策略（可靠性/历史/队列深度）。
2. `frame_id` 默认值与多相机场景的命名约定。
