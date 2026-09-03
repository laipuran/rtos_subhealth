# RFC009 单次生理采样（单指标通用消息，每传感器独立 topic 发布）
builtin_interfaces/Time timestamp
string data_src       # 传感器来源标识（如 device_spo2 / mock_heart_rate）
string data_type      # 指标类型：spo2 / heart_rate / systolic_mmhg / diastolic_mmhg / body_temp_c / respiratory_rate
float32 data          # 采样值，单位随 data_type（血氧 %、心率 bpm、血压 mmHg、体温 ℃、呼吸 次/分钟）
bool valid            # 采样是否有效；false 表示传感器异常/缺数
