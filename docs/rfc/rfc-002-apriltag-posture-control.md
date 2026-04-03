## RFC 002: 基于 AprilTag 检测数据的机器人姿态调整协议

**状态：** 草案

**修订日期：** 2026-04-02

**摘要：** 本协议定义机器人如何根据 `RFC 001` 发布的 AprilTag 检测结果，计算姿态调整量并输出控制指令。协议重点描述误差定义、控制律、滤波与安全约束，目标是让机器人朝向目标标签并稳定对齐到期望距离与姿态。

---

### 1. 决策与范围
* **适用范围：** `ros2_ws/src/control` 或上层导航/执行控制模块中的姿态调整节点。
* **输入来源：** `/perception/apriltag_detections` 发布的检测数组。
* **输出目标：** 机器人底盘速度、云台角度或执行机构目标位姿。
* **控制对象：** 基于单个或多个 AprilTag 目标，完成“转向、接近、对正、稳定”的姿态调整。
* **数据语义模型：** 以 `id`、`distance`、`center_offset_x/y`、`roll/yaw/pitch`、`hamming` 为控制输入。

---

### 2. 输入数据与控制目标

#### 2.1 输入字段含义

字段语义、单位与取值范围默认继承 `RFC 001`；下表给出本 RFC 实现时直接使用的约束。

| 字段 | 单位/范围 | 作用 |
| :--- | :--- | :--- |
| `id` | 非负整数 | 目标标签编号，用于区分不同任务点位或优先级。 |
| `distance` | mm，`> 0` | 与目标的直线距离，决定前进/后退量。 |
| `center_offset_x` | 归一化，`[-1, 1]` | 水平偏差，决定转向或横移。 |
| `center_offset_y` | 归一化，`[-1, 1]` | 垂直偏差，用于上下微调。 |
| `roll` | deg（角度） | 目标平面翻滚角，可用于姿态校正。 |
| `yaw` | deg（角度） | 目标偏航角，用于转向对齐。 |
| `pitch` | deg（角度） | 目标俯仰角，用于上下对正。 |
| `hamming` | 非负整数，越小越可信 | 置信度判定指标，用于过滤低质量检测。 |

#### 2.2 控制目标
控制器应把机器人状态从“未对齐”收敛到“目标对齐”状态。对单个目标，可将目标状态定义为：
$$
\mathbf{e} = [e_d, e_x, e_y, e_r, e_yaw, e_p]^T \to \mathbf{0}
$$
其中 $e_d$ 为距离误差，$e_x$ 为水平误差，$e_y$ 为垂直误差，$e_r$ 为翻滚误差，$e_{yaw}$ 为偏航误差，$e_p$ 为俯仰误差。

---

### 3. 场景化控制规则

本节直接给出“在什么情况下，机器人怎么走、调用哪个动作组”。
动作组均来自 `https://github.com/Hiwonder/TonyPi/tree/main/ActionGroups`。

#### 3.1 判定输入
每个控制周期只用以下信息做决策：
1. 目标是否可见（`detections` 是否为空）。
2. 目标是否可信（`hamming <= h_max`）。
3. 目标在画面中的左右偏差（`center_offset_x`）。
4. 目标距离（`distance`，毫米）。

#### 3.2 核心状态机
1. 找目标（Search）
2. 转向（Turn）
3. 接近（Approach）
4. 对正（Align）
5. 稳定（Stable）

#### 3.3 场景到动作组映射
| 场景 | 触发条件（示例） | 动作策略 | 推荐动作组 |
| :--- | :--- | :--- | :--- |
| 目标丢失 | `detections` 为空或超时 | 左右小角度扫描搜索 | `turn_left_small_step` 与 `turn_right_small_step` 交替 |
| 目标在右边 | `center_offset_x > x_turn` | 先右转再判断 | `turn_right_small_step`（偏差大可用 `turn_right`） |
| 目标在左边 | `center_offset_x < -x_turn` | 先左转再判断 | `turn_left_small_step`（偏差大可用 `turn_left`） |
| 方向基本对正但距离远 | `abs(center_offset_x) <= x_turn` 且 `distance > d_far` | 向前接近 | `go_forward_start` -> `go_forward` |
| 中距离接近 | `d_near < distance <= d_far` | 常速接近 | `go_forward` |
| 接近末段 | `d_stop_max < distance <= d_near` | 小步接近，避免冲过头 | `go_forward_one_small_step` |
| 距离过近 | `distance < d_stop_min` | 后退拉开安全距离 | `back_one_step`（必要时 `back`） |
| 已对正且到达工作距离 | `abs(center_offset_x) <= x_align` 且 `d_stop_min <= distance <= d_stop_max` | 停止并保持稳定 | `go_forward_end` -> `stand` 或 `stand_slow` |
| 低置信度 | `hamming > h_max` | 不执行激烈动作，保持姿态 | `stand` |

说明：
1. 若项目中开启侧移能力，可在对正阶段用 `left_move_10` / `right_move_10` 做微调。
2. 转向阶段优先“小步转”，只有偏差很大时才切换普通转向，降低抖动。
3. 所有连续动作建议分段执行（起步 -> 主动作 -> 收尾），减少跌倒风险。

---

### 4. 多目标选择
同一帧有多个目标时，按以下顺序选一个：
1. 先过滤 `hamming > h_max` 的目标。
2. 若配置了任务 `id`，优先选指定 `id`。
3. 再选“画面更居中”的目标（`abs(center_offset_x)` 最小）。
4. 最后用距离最近作为并列打破条件。

---

### 5. 平滑与稳定策略（工程规则）
1. **防抖：** 连续 `n_confirm` 帧满足同一判定再切动作。
2. **最短保持：** 每个动作组至少执行 `t_hold` 后再允许切换。
3. **慢停优先：** 进入稳定态时优先 `go_forward_end` + `stand_slow`。
4. **异常回退：** 若连续 `n_lost` 帧丢目标，强制切回“找目标”。
5. **安全上锁：** 低置信度时只允许 `stand`、`turn_*_small_step` 这类保守动作。

---

### 6. 搜索执行规则（跨 tick 交替）
为避免同一周期内左右动作互相抵消，搜索状态按以下规则执行：
1. 每个控制 tick 只允许执行一个搜索动作。
2. 维护 `search_dir` 状态（`left` 或 `right`）。
3. 本 tick 执行 `search_dir` 指定方向后，翻转 `search_dir`，下一 tick 再执行另一方向。
4. 若重新检测到有效目标，立即退出搜索并进入转向/接近状态。

---

### 7. 安全与降级策略
1. **空数组处理：** 若 `detections` 为空，输出搜索/保持命令，不进入激烈控制。  
2. **超时处理：** 若消息超时，则将控制量降为零或保持上一稳定状态。  
3. **异常处理：** 若检测结果数值异常或参数非法，则忽略该帧并记录告警。  
4. **限幅保护：** 所有速度、角速度、舵机角均必须经过饱和。  
5. **低置信度保护：** `hamming > h_{max}` 时不执行大角度纠偏。

---

### 8. 参数建议（节点级）
| 参数名 | 默认值 | 说明 |
| :--- | :--- | :--- |
| `d_far` | `900` | 远距离阈值（mm），大于此值进入快速接近 |
| `d_near` | `750` | 近距离阈值（mm），小于等于此值切小步接近 |
| `d_stop_min` | `520` | 目标最小工作距离（mm） |
| `d_stop_max` | `650` | 目标最大工作距离（mm） |
| `h_max` | `2` | 最大可接受汉明距离 |
| `x_turn` | `0.08` | 进入转向的水平偏差阈值 |
| `x_align` | `0.03` | 认为对正完成的水平偏差阈值 |
| `n_confirm` | `3` | 动作切换前的连续确认帧数 |
| `n_lost` | `5` | 连续丢目标回退搜索的帧数 |
| `t_hold` | `0.25` | 单个动作最短保持时间（s） |

---

### 9. 验收标准
1. **可理解性：** 开发者可直接按“场景 -> 动作组”实现，不依赖复杂控制公式。  
2. **对齐性：** 目标在左/右时，机器人能通过 `turn_left*` / `turn_right*` 完成方向对正。  
3. **接近性：** 目标偏远时能前进接近，过近时能后退，最终停在 `[d_stop_min, d_stop_max]`。  
4. **稳定性：** 到位后进入 `stand` 或 `stand_slow`，不反复抖动切动作。  
5. **安全性：** 空检测、低置信度、超时时不出现激烈动作。

---

### 10. 建议实现流程
1. 订阅 `/perception/apriltag_detections`。  
2. 过滤空数组、超时数据与高 `hamming` 数据。  
3. 根据任务 `id` 与居中程度选择单一目标。  
4. 先判断左右偏差，决定转向（`turn_left*` / `turn_right*`）。  
5. 再判断距离区间，决定接近或后退（`go_forward*` / `back*`）。  
6. 达到对正与距离窗口后，执行收尾并站稳（`go_forward_end` + `stand_slow`）。  
7. 循环执行，若丢目标则回到搜索状态。

#### 10.1 控制链路伪代码
```text
on_control_tick():
	msg = latest_apriltag_msg()

	# search_dir: 持久状态，left 或 right
	if msg is None or timeout(msg):
		if search_dir == "left":
			run_action("turn_left_small_step")
			search_dir = "right"
		else:
			run_action("turn_right_small_step")
			search_dir = "left"
		return

	target = select_target(msg.detections, h_max, allowed_ids)
	if target is None:
		if search_dir == "left":
			run_action("turn_left_small_step")
			search_dir = "right"
		else:
			run_action("turn_right_small_step")
			search_dir = "left"
		return

	x = target.center_offset_x
	d = target.distance  # mm

	if target.hamming > h_max:
		run_action("stand")
		return

	if x > x_turn:
		run_action("turn_right_small_step")
		return

	if x < -x_turn:
		run_action("turn_left_small_step")
		return

	if d > d_far:
		run_action_sequence(["go_forward_start", "go_forward"])
		return

	if d_near < d <= d_far:
		run_action("go_forward")
		return

	if d_stop_max < d <= d_near:
		run_action("go_forward_one_small_step")
		return

	if d < d_stop_min:
		run_action("back_one_step")
		return

	# 对正且距离合适：进入稳定态
	run_action_sequence(["go_forward_end", "stand_slow"])
```

---

### 11. 参考
* [RFC 001: AprilTag 感知发布协议](rfc-001-apriltag.md)
