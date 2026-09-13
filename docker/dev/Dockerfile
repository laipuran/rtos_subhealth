# Jazzy + Rust development image. Build once; source is bind-mounted at runtime.
#
#   docker build -t ros-dev:jazzy docker/dev
#   docker run --rm -it --network host -v "$PWD":/workspace ros-dev:jazzy
#
# All ROS/Rust/C++ tooling is provided by the container; nothing is installed
# on the host.
FROM ros:jazzy-ros-base

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential cmake git curl pkg-config libclang-dev \
      python3-pip python3-vcstool \
      ros-jazzy-rmw-cyclonedds-cpp \
      ros-jazzy-cv-bridge \
      ros-jazzy-sensor-msgs \
      ros-jazzy-rosidl-default-generators \
      ros-jazzy-rosidl-default-runtime \
      ros-jazzy-test-msgs \
      ros-jazzy-example-interfaces \
 && rm -rf /var/lib/apt/lists/*

# Pinned Rust toolchain (matches rust-toolchain.toml and CI).
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain 1.85.0 --profile minimal \
                 --component rustfmt --component clippy
ENV PATH="/root/.cargo/bin:${PATH}"

# colcon plugins that teach colcon to build Cargo packages (rclrs).
RUN pip install --no-cache-dir --break-system-packages colcon-cargo colcon-ros-cargo || true

ENV RMW_IMPLEMENTATION=rmw_cyclonedds_cpp
ENV ROS_DOMAIN_ID=1

WORKDIR /workspace
CMD ["bash"]
