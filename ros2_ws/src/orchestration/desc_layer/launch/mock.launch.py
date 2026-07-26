"""Launch mock mode: mock_exec_layer + planner + desc_layer.

Usage:
  # 默认连 mock_exec_task（mock 模式）
  ros2 launch desc_layer mock.launch.py

  # 连 exec_layer（需要 planner + exec_layer 也在运行）
  ros2 launch desc_layer mock.launch.py exec_action_name:=exec_task
"""
import os

from launch import LaunchDescription
from launch.actions import SetEnvironmentVariable, LogInfo, DeclareLaunchArgument
from launch.substitutions import LaunchConfiguration
from launch_ros.actions import Node


CURI = '<CycloneDDS><Domain><General><NetworkInterfaceAddress>lo</NetworkInterfaceAddress></General></Domain></CycloneDDS>'

def generate_launch_description():
    ws_dir = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..")
    ws_dir = os.path.normpath(ws_dir)
    params_dir = os.path.join(ws_dir, "config", "params")
    maps_dir = os.path.join(ws_dir, "config", "maps")

    return LaunchDescription([
        DeclareLaunchArgument("exec_action_name", default_value="mock_exec_task",
                              description="target action server"),

        SetEnvironmentVariable("RMW_IMPLEMENTATION", "rmw_cyclonedds_cpp"),
        SetEnvironmentVariable("ROS_DOMAIN_ID", "1"),
        SetEnvironmentVariable("CYCLONEDDS_URI", CURI),

        LogInfo(msg=f"[mock.launch] ws_dir={ws_dir}"),

        Node(
            package="mock_exec_layer",
            executable="mock_exec_layer_node",
            name="mock_exec_layer",
            output="screen",
        ),
        Node(
            package="planner",
            executable="planner_node",
            name="planner",
            output="screen",
        ),
        Node(
            package="desc_layer",
            executable="desc_layer_node",
            name="desc_layer",
            parameters=[
                os.path.join(params_dir, "desc_layer_dev.yaml"),
                {"maps_dir": maps_dir,
                 "exec_action_name": LaunchConfiguration("exec_action_name")},
            ],
            output="screen",
        ),
    ])
