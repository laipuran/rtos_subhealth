# RFC009 实时体征流（诊断层周期性发布，desc_layer 经 WS 广播给 WebUI）
builtin_interfaces/Time timestamp
DiagnosisMetric[] metrics   # 各传感器的当前窗口统计
