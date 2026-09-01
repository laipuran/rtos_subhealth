"""统一启动入口：mock / sim / real 三种后端。

Usage:
  ros2 launch desc_layer run.launch.py backend:=mock
  ros2 launch desc_layer run.launch.py backend:=sim
  ros2 launch desc_layer run.launch.py backend:=real
"""
import os

from launch import LaunchDescription
from launch.actions import SetEnvironmentVariable, LogInfo, DeclareLaunchArgument, OpaqueFunction
from launch.substitutions import LaunchConfiguration
from launch_ros.actions import Node


CURI = '<CycloneDDS><Domain><General><NetworkInterfaceAddress>lo</NetworkInterfaceAddress></General></Domain></CycloneDDS>'


def _make_nodes(context, *args, **kwargs):
    """解析 backend 参数并构建节点列表。"""
    backend = LaunchConfiguration("backend").perform(context)
    maps_dir = os.path.join(os.getcwd(), "config", "maps")

    nodes = [
        # Planner（所有模式都启动）
        Node(
            package="exec_layer",
            executable="planner_node",
            name="planner",
            parameters=[{"maps_dir": maps_dir}],
            output="screen",
        ),
    ]

    if backend == "mock":
        nodes += [
            Node(
                package="mock_exec_layer",
                executable="mock_exec_layer_node",
                name="mock_exec_layer",
                output="screen",
            ),
        ]
        exec_action = "mock_exec_task"
    else:
        nodes += [
            Node(
                package="exec_layer",
                executable="exec_layer_node",
                name="exec_layer",
                parameters=[{"robot_backend": backend}],
                output="screen",
            ),
        ]
        exec_action = "exec_task"

    nodes += [
        Node(
            package="desc_layer",
            executable="desc_layer_node",
            name="desc_layer",
            parameters=[
                {"maps_dir": maps_dir},
                {"exec_action_name": exec_action},
                {"http_port": 5000},
            ],
            output="screen",
        ),
    ]

    # RFC-009 生理传感 + 诊断层
    nodes += [
        Node(
            package="physio_mock_publisher",
            executable="physio_mock_publisher_node",
            name="physio_mock_publisher",
            parameters=[{"scenario": "normal"}],
            output="screen",
        ),
        Node(
            package="diagnosis_layer",
            executable="diagnosis_layer_node",
            name="diagnosis_layer",
            parameters=[
                {"medical_corpus_dir": ""},
                {"rag_top_k": 3},
                {"confidence_min": 0.8},
                {"llm_base_url": ""},
                {"llm_api_key": ""},
                {"llm_model": ""},
                {"embedding_base_url": ""},
            ],
            output="screen",
        ),
    ]
    return nodes


def generate_launch_description():
    return LaunchDescription([
        DeclareLaunchArgument("backend", default_value="mock",
                              description="mock | sim | real"),

        SetEnvironmentVariable("RMW_IMPLEMENTATION", "rmw_cyclonedds_cpp"),
        SetEnvironmentVariable("ROS_DOMAIN_ID", "1"),
        SetEnvironmentVariable("CYCLONEDDS_URI", CURI),

        LogInfo(msg=f"[run.launch] backend={LaunchConfiguration('backend')}"),

        OpaqueFunction(function=_make_nodes),
    ])
