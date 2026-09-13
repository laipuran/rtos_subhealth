# RFC009 单个生理指标聚合统计（用于诊断结果与实时体征流）
# 单位随 data_type：血氧 %、心率 bpm、血压 mmHg、体温 ℃、呼吸 次/分钟
string data_src        # 传感器来源标识（如 mock_spo2）
string data_type       # 指标类型：spo2 / heart_rate / systolic_mmhg / ...
float32 latest         # 窗口内最新采样值
float32 mean           # 窗口内均值
float32 min            # 窗口内最小值
float32 max            # 窗口内最大值
string trend           # increasing | decreasing | stable | unknown
bool valid             # 窗口内是否有有效样本
