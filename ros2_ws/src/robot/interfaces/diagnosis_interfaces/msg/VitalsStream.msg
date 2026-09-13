# RFC-009 live vitals stream (published periodically by the diagnosis layer).
builtin_interfaces/Time timestamp
DiagnosisMetric[] metrics  # current window statistics for every sensor
