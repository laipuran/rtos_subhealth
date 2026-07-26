"""Launch production mode.

Usage:
  ros2 launch desc_layer production.launch.py robot_backend:=mock
  ros2 launch desc_layer production.launch.py robot_backend:=sim
  ros2 launch desc_layer production.launch.py robot_backend:=real api_token:=my-secret
  ros2 launch desc_layer production.launch.py exec_action_name:=exec_task
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
        DeclareLaunchArgument("robot_backend", default_value="mock",
                              description="mock|sim|real"),
        DeclareLaunchArgument("api_token", default_value="",
                              description="API auth token, empty=disabled"),
        DeclareLaunchArgument("exec_action_name", default_value="exec_task",
                              description="target action server"),

        SetEnvironmentVariable("RMW_IMPLEMENTATION", "rmw_cyclonedds_cpp"),
        SetEnvironmentVariable("ROS_DOMAIN_ID", "1"),
        SetEnvironmentVariable("CYCLONEDDS_URI", CURI),

        LogInfo(msg=f"[production.launch] ws_dir={ws_dir}"),

        Node(
            package="planner",
            executable="planner_node",
            name="planner",
            parameters=[os.path.join(params_dir, "planner.yaml")],
            output="screen",
        ),
        Node(
            package="exec_layer",
            executable="exec_layer_node",
            name="exec_layer",
            parameters=[{"robot_backend": LaunchConfiguration("robot_backend")}],
            output="screen",
        ),
        Node(
            package="desc_layer",
            executable="desc_layer_node",
            name="desc_layer",
            parameters=[
                os.path.join(params_dir, "desc_layer_prod.yaml"),
                {"maps_dir": maps_dir,
                 "api_token": LaunchConfiguration("api_token"),
                 "exec_action_name": LaunchConfiguration("exec_action_name")},
            ],
            output="screen",
        ),
    ])
