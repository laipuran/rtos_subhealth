"""Launch mock mode: mock_exec_layer + desc_layer + planner."""
import os

from launch import LaunchDescription
from launch_ros.actions import Node


def generate_launch_description():
    maps_dir = os.path.join(os.getcwd(), "config", "maps")

    return LaunchDescription([
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
