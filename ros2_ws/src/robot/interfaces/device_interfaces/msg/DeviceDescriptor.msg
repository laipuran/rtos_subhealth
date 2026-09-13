# Capability advertisement published by every device adapter (transient-local).
# Devices are NOT assumed to be mobile or to provide odometry (e.g. a servo-based
# humanoid may only support named action groups).
string device_id
string name
# wheeled_diff | wheeled_omni | legged | aerial | arm | mock
string kind
# Semantic capabilities, matched by the orchestrator:
#   holonomic | rotate_in_place | can_align | can_dock | supports_posture
#   supports_velocity | supports_pose | supports_action_groups
string[] capabilities
# Primitive names this device can execute, e.g.
#   move_to_pose | set_velocity | hold | stop | execute_primitive
string[] primitives
DeviceLimits limits
# Coordinate frames the device understands (e.g. map, odom, base_link).
string[] frames
# Available sensors (e.g. imu, battery, camera, odometry).
string[] sensors
