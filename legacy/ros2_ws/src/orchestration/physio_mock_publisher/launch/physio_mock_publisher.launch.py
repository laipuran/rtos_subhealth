"""Launch physio_mock_publisher node.

Usage:
  ros2 launch physio_mock_publisher physio_mock_publisher.launch.py
  ros2 launch physio_mock_publisher physio_mock_publisher.launch.py scenario:=anomaly
"""
from launch import LaunchDescription
from launch.actions import DeclareLaunchArgument
from launch.substitutions import LaunchConfiguration
from launch_ros.actions import Node


def generate_launch_description():
    scenario = LaunchConfiguration("scenario")

    return LaunchDescription([
        DeclareLaunchArgument("scenario", default_value="normal",
                              description="normal | anomaly"),

        Node(
            package="physio_mock_publisher",
            executable="physio_mock_publisher_node",
            name="physio_mock_publisher",
            parameters=[{"scenario": scenario}],
            output="screen",
        ),
    ])
