#!/usr/bin/env python3
"""Wrapper to run a ROS2 node without pkg_resources entry points.

Usage: python3 run_node.py <node_name> [--ros-args ...]
"""
import os, sys

ws = os.path.normpath(os.path.join(os.path.dirname(__file__)))
for d in ["install/exec_layer/lib/python3.8/site-packages",
           "install/desc_layer/lib/python3.8/site-packages"]:
    p = os.path.join(ws, d)
    if os.path.isdir(p) and p not in sys.path:
        sys.path.insert(0, p)

os.environ.setdefault("RCUTILS_CONSOLE_OUTPUT_FORMAT",
                       '[{severity}][{name}]: {message}')

node_name = sys.argv[1]
ros_args = sys.argv[2:]
sys.argv = [node_name] + ros_args

if node_name == "planner_node":
    from exec_layer.planner.planner_node import main
elif node_name == "exec_layer_node":
    from exec_layer.exec_layer_node import main
elif node_name == "desc_layer_node":
    from desc_layer.desc_layer_node import main
else:
    print(f"Unknown node: {node_name}", file=sys.stderr)
    sys.exit(1)

import rclpy
rclpy.init(args=sys.argv)
try:
    main()
except KeyboardInterrupt:
    pass
finally:
    rclpy.shutdown()
