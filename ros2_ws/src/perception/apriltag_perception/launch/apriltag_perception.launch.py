import os

from ament_index_python.packages import get_package_share_directory
from launch import LaunchDescription
from launch_ros.actions import Node


def generate_launch_description() -> LaunchDescription:
    config_file = os.path.join(
        get_package_share_directory("apriltag_perception"),
        "config",
        "apriltag_params.yaml",
    )

    return LaunchDescription(
        [
            Node(
                package="apriltag_perception",
                executable="apriltag_perception_node",
                name="apriltag_perception_node",
                output="screen",
                parameters=[config_file],
            )
        ]
    )
