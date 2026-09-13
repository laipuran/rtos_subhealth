# RFC-009 structured multi-source diagnosis result.
builtin_interfaces/Time timestamp
string diagnosis_id
string[] source_ids        # data_src values participating in this diagnosis
string trigger_type        # periodic | anomaly | manual
string severity            # normal | mild | moderate | severe | critical
string summary
string[] possible_causes
string[] recommendations
float32 confidence
string disclaimer
string raw_prompt
string error_code
string error_message
DiagnosisMetric[] metrics  # per-source window statistics
