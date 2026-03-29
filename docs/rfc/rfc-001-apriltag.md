## RFC 001: AprilTag 感知发布协议

**状态：** 草案

**修订日期：** 2026-03-29

**摘要：** 本协议定义 perception 模块的 AprilTag 检测结果发布规范。该规范用于 ROS2 场景，输入为真实相机图像，输出为固定 topic 的结构化消息。

---

### 1. 决策与范围
* **适用范围：** `ros2_ws/src/perception` 内 AprilTag 检测发布节点。
* **输入来源：** 真实相机图像 topic（参数化输入）。
* **输出 topic（固定）：** `/perception/apriltag_detections`。
* **消息承载方式：** ROS2 自定义消息（结构化字段）。
* **数据语义模型：** 语义与本 RFC 定义的 JSON 模型等价。
* **建议发布频率：** $\ge 10$ Hz。

---

### 2. ROS2 消息字段映射

#### 2.1 检测项（单目标）
| 字段 | 类型 | 单位 | 说明 |
| :--- | :--- | :--- | :--- |
| `id` | int32 | - | AprilTag 编号，映射医疗点位（如 0=药房, 1=病床） |
| `distance` | float32/float64 | mm | 相机中心到 Tag 中心直线距离 |
| `center_offset_x` | float32 | 归一化 | 水平偏差，范围 $[-1.0, 1.0]$ |
| `center_offset_y` | float32 | 归一化 | 垂直偏差，范围 $[-1.0, 1.0]$ |
| `roll` | float32 | 度 | 翻滚角 |
| `yaw` | float32 | 度 | 偏航角 |
| `pitch` | float32 | 度 | 俯仰角 |
| `hamming` | int32 | - | 汉明距离，`0` 为完美匹配 |

#### 2.2 检测数组（一帧）
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `timestamp` | builtin_interfaces/Time | ROS 时间戳 |
| `frame_id` | string | 相机坐标系（默认 `camera_link`，可参数化） |
| `detections` | 检测项数组 | 当前帧所有目标；无目标时为空数组 |

---

### 3. 实施要求
1. **坐标归一化：** `center_offset_x/y` 必须按图像分辨率归一化到 $[-1.0, 1.0]$。
2. **空集持续发布：** 未识别到目标时必须发布空数组，不允许停止发布。
3. **频率下限：** 发布频率必须满足 $\ge 10$ Hz。
4. **单位约束：** `distance` 统一为 mm；姿态角统一为度。
5. **鲁棒性要求：** 图像空帧、检测异常、参数非法时节点不可崩溃，需降级并持续发布。

---

### 4. 参数建议（节点级）
| 参数名 | 默认值 | 说明 |
| :--- | :--- | :--- |
| `input_image_topic` | `/camera/image_raw` | 相机输入 topic |
| `output_topic` | `/perception/apriltag_detections` | 输出 topic（建议固定，仅调试场景可覆写） |
| `frame_id` | `camera_link` | 输出坐标系 |
| `publish_rate_hz` | `10.0` | 发布频率 |
| `tag_family` | `tag36h11` | AprilTag family |
| `tag_size_m` | `0.16` | 标签物理边长（米） |
| `hamming_max` | `2` | 接收的最大汉明距离 |

---

### 5. 验收标准
1. **接口一致性：** 字段、单位、取值范围与本 RFC 一致。
2. **频率达标：** 在目标设备上 topic 发布频率稳定不低于 10Hz。
3. **空集行为：** 无目标场景持续输出空数组。
4. **边界行为：** 目标位于画面边缘时，`center_offset_x/y` 不越界。
5. **稳定性：** 连续运行过程中，输入短时抖动不导致节点退出。

---