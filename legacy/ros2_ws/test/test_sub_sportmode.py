"""订阅 rt/sportmodestate 验证 DDS 反馈通道

用法:
  # 终端 A: 启动 MuJoCo 仿真
  cd ~/unitree_mujoco/simulate_python && python3 unitree_mujoco.py

  # 终端 B: 运行本脚本
  source ros2_ws/setup.sh
  python3 ros2_ws/test/test_sub_sportmode.py
"""
import sys
import os
import time

sdk_path = os.path.expanduser("~/unitree_sdk2_python")
if os.path.isdir(sdk_path) and sdk_path not in sys.path:
    sys.path.insert(0, sdk_path)

from unitree_sdk2py.core.channel import ChannelFactoryInitialize, ChannelSubscriber
from unitree_sdk2py.idl.unitree_go.msg.dds_ import SportModeState_


received = 0
start_time = None


def handler(msg):
    global received, start_time
    if start_time is None:
        start_time = time.time()
    received += 1
    elapsed = time.time() - start_time
    pos = msg.position
    vel = msg.velocity
    imu = msg.imu_state
    print(
        f"[{elapsed:6.2f}s]  #{received:4d}"
        f"  pos=({pos[0]:6.2f}, {pos[1]:6.2f}, {pos[2]:6.2f})"
        f"  vel=({vel[0]:6.2f}, {vel[1]:6.2f}, {vel[2]:6.2f})"
        f"  imu_quat=({imu.quaternion[0]:.2f}, {imu.quaternion[1]:.2f}, {imu.quaternion[2]:.2f}, {imu.quaternion[3]:.2f})"
    )


def main():
    global start_time
    print("[test] Initializing DDS...")
    ChannelFactoryInitialize(1, "lo")
    print("[test] Subscribing to rt/sportmodestate...")

    sub = ChannelSubscriber("rt/sportmodestate", SportModeState_)
    sub.Init(handler, 10)

    print("[test] Listening for 10 seconds...")
    start_time = time.time()
    time.sleep(10)

    print(f"\n[test] Done. Received {received} messages in 10 seconds ({received / 10:.0f} Hz)")


if __name__ == "__main__":
    main()
