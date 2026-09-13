# RFC-009 single physiological sample (one generic message per metric).
builtin_interfaces/Time timestamp
string data_src       # sensor source id (e.g. device_spo2 / mock_heart_rate)
string data_type      # spo2 / heart_rate / systolic_mmhg / diastolic_mmhg / body_temp_c / respiratory_rate
float32 data          # sample value; unit depends on data_type
bool valid            # false means sensor fault / missing data
