"""Launch production mode: exec_layer + planner + desc_layer + perception."""
import os

from launch import LaunchDescription
from launch_ros.actions import Node


def generate_launch_description():
    maps_dir = os.path.join(os.getcwd(), "config", "maps")

    return LaunchDescription([
        Node(
            package="planner",
            executable="planner_node",
            name="planner",
            parameters=[{"maps_dir": maps_dir}],
            output="screen",
        ),
        Node(
            package="exec_layer",
            executable="exec_layer_node",
            name="exec_layer",
            parameters=[{"robot_backend": "mock"}],
            output="screen",
        ),
        Node(
            package="desc_layer",
            executable="desc_layer_node",
            name="desc_layer",
            parameters=[
                {"maps_dir": maps_dir},
                {"exec_action_name": "exec_task"},
                {"http_port": 5000},
            ],
            output="screen",
        ),
    ])
