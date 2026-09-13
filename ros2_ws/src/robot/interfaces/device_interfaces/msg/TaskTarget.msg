# Device-agnostic task target. The active interpretation is selected by `kind`.
string kind                 # tag | pose | waypoint | action
int32 tag_id                # kind=tag
string waypoint_id          # kind=waypoint
geometry_msgs/PoseStamped pose  # kind=pose
string action_id            # kind=action (device-specific, e.g. TonyPi action group)
float32 position_tolerance_m
float32 yaw_tolerance_rad
