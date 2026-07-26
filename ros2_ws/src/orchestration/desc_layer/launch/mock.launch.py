"""Launch mock mode: mock_exec_layer + planner + desc_layer.

Usage:
  source ros2_ws/setup.sh
  ros2 launch desc_layer mock.launch.py
"""
import os

from launch import LaunchDescription
from launch.actions import SetEnvironmentVariable, LogInfo
from launch_ros.actions import Node


CURI = '<CycloneDDS><Domain><General><NetworkInterfaceAddress>lo</NetworkInterfaceAddress></General></Domain></CycloneDDS>'


def generate_launch_description():
    maps_dir = os.path.join(os.getcwd(), "config", "maps")

    return LaunchDescription([
        SetEnvironmentVariable("RMW_IMPLEMENTATION", "rmw_cyclonedds_cpp"),
        SetEnvironmentVariable("ROS_DOMAIN_ID", "1"),
        SetEnvironmentVariable("CYCLONEDDS_URI", CURI),

        LogInfo(msg=f"[mock.launch] maps_dir={maps_dir}"),

        Node(
            package="mock_exec_layer",
            executable="mock_exec_layer_node",
            name="mock_exec_layer",
            output="screen",
        ),
        Node(
            package="exec_layer",
            executable="planner_node",
            name="planner",
            parameters=[{"maps_dir": maps_dir}],
            output="screen",
        ),
        Node(
            package="desc_layer",
            executable="desc_layer_node",
            name="desc_layer",
            parameters=[
                {"maps_dir": maps_dir},
                {"exec_action_name": "mock_exec_task"},
                {"http_port": 5000},
            ],
            output="screen",
        ),
    ])
