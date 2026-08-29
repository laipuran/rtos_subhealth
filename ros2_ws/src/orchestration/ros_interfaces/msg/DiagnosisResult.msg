# RFC009 结构化诊断结果（多源，由 diagnosis_layer 发布，desc_layer 持久化/广播）
builtin_interfaces/Time timestamp
string diagnosis_id
string[] source_ids        # 参与本次诊断的传感器来源集合（data_src）
string trigger_type       # periodic | anomaly | manual
string severity           # normal | mild | moderate | severe | critical
string summary
string[] possible_causes
string[] recommendations
float32 confidence
string disclaimer
string raw_prompt
string error_code
string error_message
