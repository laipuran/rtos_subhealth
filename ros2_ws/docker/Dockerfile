# ROS2 Foxy 编译/运行环境（一次性构建，源码改动不触发重建）
# 基座与开发环境一致：Ubuntu 20.04 + ROS2 Foxy
FROM ros:foxy-ros-base-focal

# --- 包管理器 ---
ENV DEBIAN_FRONTEND=noninteractive

# --- 编译工具链（编译 ament_cmake 接口包 + colcon）---
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3-colcon-common-extensions \
    build-essential \
    cmake \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# --- ROS 二进制依赖 ---
# run.launch.py / setup.sh 强制 RMW=rmw_cyclonedds_cpp，需要 CycloneDDS
# 其余为接口包与 perception 基础依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    ros-foxy-rmw-cyclonedds-cpp \
    ros-foxy-cv-bridge \
    ros-foxy-sensor-msgs \
    ros-foxy-rosidl-default-generators \
    ros-foxy-rosidl-default-runtime \
    ros-foxy-launch \
    ros-foxy-launch-ros \
    ros-foxy-ament-index-python \
    ros-foxy-builtin-interfaces \
    python3-opencv \
    python3-numpy \
    && rm -rf /var/lib/apt/lists/*

# --- Python 运行时依赖（mock 链路 + 备用）---
RUN python3 -m pip install --no-cache-dir \
    flask \
    flask-sock \
    transitions

# --- diagnosis_layer 运行时依赖（RFC-009：requests / numpy / pytest）---
# diagnosis_layer/setup.py 声明 install_requires=["requests>=2.20"]，缺会后入口脚本
# 在 pkg_resources.require 处抛分布未找到而崩溃，必须随镜像安装。
RUN python3 -m pip install --no-cache-dir \
    requests \
    numpy \
    pytest

# 非 root 用户（1000 = 大多数桌面主机的当前用户），避免文件属主问题
RUN groupadd -g 1000 rosdev && useradd -m -u 1000 -g rosdev -s /bin/bash rosdev

WORKDIR /workspace
USER rosdev
