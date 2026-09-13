cmake_minimum_required(VERSION 3.8)
project(diagnosis_interfaces)

find_package(ament_cmake REQUIRED)
find_package(rosidl_default_generators REQUIRED)
find_package(builtin_interfaces REQUIRED)

rosidl_generate_interfaces(${PROJECT_NAME}
  "msg/PhysioSample.msg"
  "msg/VitalsStream.msg"
  "msg/DiagnosisResult.msg"
  "msg/DiagnosisMetric.msg"
  DEPENDENCIES builtin_interfaces
)

ament_export_dependencies(rosidl_default_runtime)
ament_package()
