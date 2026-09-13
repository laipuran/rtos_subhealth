cmake_minimum_required(VERSION 3.8)
project(task_interfaces)

find_package(ament_cmake REQUIRED)
find_package(rosidl_default_generators REQUIRED)
find_package(builtin_interfaces REQUIRED)
find_package(device_interfaces REQUIRED)

rosidl_generate_interfaces(${PROJECT_NAME}
  "action/ExecTask.action"
  "action/DeviceTask.action"
  "srv/PlanPath.srv"
  "msg/Segment.msg"
  "msg/Constraints.msg"
  DEPENDENCIES builtin_interfaces device_interfaces
)

ament_export_dependencies(rosidl_default_runtime)
ament_package()
