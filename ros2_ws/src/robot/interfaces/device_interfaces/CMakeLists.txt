cmake_minimum_required(VERSION 3.8)
project(device_interfaces)

find_package(ament_cmake REQUIRED)
find_package(rosidl_default_generators REQUIRED)
find_package(builtin_interfaces REQUIRED)
find_package(geometry_msgs REQUIRED)

rosidl_generate_interfaces(${PROJECT_NAME}
  "msg/DeviceLimits.msg"
  "msg/DeviceDescriptor.msg"
  "msg/DeviceState.msg"
  "msg/TaskTarget.msg"
  "action/MoveToPose.action"
  "action/Hold.action"
  "action/ExecutePrimitive.action"
  "srv/SetVelocity.srv"
  "srv/Stop.srv"
  DEPENDENCIES builtin_interfaces geometry_msgs
)

ament_export_dependencies(rosidl_default_runtime)
ament_package()
