import os

from ament_index_python.packages import get_package_share_directory
from launch import LaunchDescription
from launch_ros.actions import Node


def generate_launch_description() -> LaunchDescription:
    config_file = os.path.join(
        get_package_share_directory("camera_test_publisher"),
        "config",
        "camera_test_params.yaml",
    )

    return LaunchDescription(
        [
            Node(
                package="camera_test_publisher",
                executable="camera_test_node",
                name="camera_test_node",
                output="screen",
                parameters=[config_file],
            )
        ]
    )
