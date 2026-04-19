import os

from ament_index_python.packages import get_package_share_directory
from launch import LaunchDescription
from launch.actions import DeclareLaunchArgument, SetEnvironmentVariable
from launch.substitutions import LaunchConfiguration
from launch_ros.actions import Node


def generate_launch_description() -> LaunchDescription:
    package_share = get_package_share_directory("apriltag_perception")
    config_file = os.path.join(
        package_share,
        "config",
        "apriltag_test_params.yaml",
    )
    default_cyclonedds_uri = f"file://{os.path.join(get_package_share_directory('camera_test_publisher'), 'config', 'cyclonedds_eth1.xml')}"

    return LaunchDescription(
        [
            DeclareLaunchArgument(
                "rmw_implementation",
                default_value="rmw_cyclonedds_cpp",
                description="ROS 2 RMW implementation used by apriltag node.",
            ),
            DeclareLaunchArgument(
                "cyclonedds_uri",
                default_value=default_cyclonedds_uri,
                description="CycloneDDS XML URI (file://...) for apriltag node.",
            ),
            SetEnvironmentVariable(
                name="RMW_IMPLEMENTATION",
                value=LaunchConfiguration("rmw_implementation"),
            ),
            SetEnvironmentVariable(
                name="CYCLONEDDS_URI",
                value=LaunchConfiguration("cyclonedds_uri"),
            ),
            Node(
                package="apriltag_perception",
                executable="apriltag_perception_node",
                name="apriltag_perception_node",
                output="screen",
                parameters=[config_file],
            )
        ]
    )
