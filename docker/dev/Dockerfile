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
      build-essential cmake git curl pkg-config libclang-dev sudo \
      python3-pip python3-vcstool \
      ros-jazzy-rmw-cyclonedds-cpp \
      ros-jazzy-cv-bridge \
      ros-jazzy-sensor-msgs \
      ros-jazzy-rosidl-default-generators \
      ros-jazzy-rosidl-default-runtime \
      ros-jazzy-test-msgs \
      ros-jazzy-example-interfaces \
 && rm -rf /var/lib/apt/lists/*

# The base image already ships an `ubuntu` user at UID/GID 1000. Reuse it as
# the non-root developer; the Dev Container `updateRemoteUserUID` setting keeps
# its UID in sync with the host user when they differ.
RUN printf 'ubuntu ALL=(ALL) NOPASSWD:ALL\n' > /etc/sudoers.d/ubuntu \
 && chmod 0440 /etc/sudoers.d/ubuntu

# Pinned Rust toolchain (matches rust-toolchain.toml), shared so both root and
# the dev user can use it.
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:${PATH}
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain 1.85.0 --profile minimal \
                 --component rustfmt --component clippy --component rust-analyzer \
 && chmod -R a+w /usr/local/cargo /usr/local/rustup

# cargo-deb for packaging the Rust services (`make deb`). Pinned to the newest
# release that still builds with the 1.85 toolchain.
RUN cargo install cargo-deb --version 2.8.0 --locked

# colcon plugins that teach colcon to build Cargo packages (rclrs).
RUN pip install --no-cache-dir --break-system-packages colcon-cargo colcon-ros-cargo || true

ENV RMW_IMPLEMENTATION=rmw_cyclonedds_cpp
ENV ROS_DOMAIN_ID=1

WORKDIR /workspace
CMD ["bash"]
