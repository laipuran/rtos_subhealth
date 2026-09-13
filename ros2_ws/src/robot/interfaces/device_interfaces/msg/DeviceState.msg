# Periodic device state. pose/velocity may be unset for devices without odometry.
builtin_interfaces/Time timestamp
string device_id
geometry_msgs/PoseStamped pose
geometry_msgs/Twist velocity
float32 battery_percent
string mode
bool healthy
string error_code
