# RFC-009 single aggregated metric (used by both diagnosis results and the live vitals stream).
string data_src        # sensor source id (e.g. mock_spo2)
string data_type       # spo2 / heart_rate / systolic_mmhg / ...
float32 latest         # latest sample in the window
float32 mean           # window mean
float32 min            # window minimum
float32 max            # window maximum
string trend           # increasing | decreasing | stable | unknown
bool valid             # whether the window has any valid sample
