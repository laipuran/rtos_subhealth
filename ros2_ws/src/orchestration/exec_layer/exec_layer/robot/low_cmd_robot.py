from __future__ import annotations

import math
import time
from typing import Optional

import numpy as np

from unitree_sdk2py.core.channel import ChannelFactoryInitialize, ChannelPublisher
from unitree_sdk2py.idl.unitree_go.msg.dds_ import LowCmd_
from unitree_sdk2py.idl.default import unitree_go_msg_dds__LowCmd_
from unitree_sdk2py.utils.crc import CRC

from .interface import RobotInterface

MOTOR_NUM = 20
JOINTS_PER_LEG = 3
NUM_LEGS = 4

# Default standing joint positions from walk_go2.py
STAND_UP_JPOS = np.array([
    0.00571868, 0.608813, -1.21763,
    -0.00571868, 0.608813, -1.21763,
    0.00571868, 0.608813, -1.21763,
    -0.00571868, 0.608813, -1.21763,
], dtype=float)

# Motor indices per leg: 0=FR, 1=FL, 2=RR, 3=RL
# Each leg: 0=abduction, 1=thigh, 2=calf
LEG_ORDER = ["FR", "FL", "RR", "RL"]

# Trot gait: diagonal pairs in sync
# FR(0) + RL(3) phase 0, FL(1) + RR(2) phase pi
TROT_PHASES = [0.0, math.pi, math.pi, 0.0]


class LowCmdRobot(RobotInterface):
    """Controls MuJoCo simulation via unitree DDS LowCmd.

    Uses low-level joint position control with a trot gait for movement,
    matching the walk_go2.py approach.
    """

    def __init__(
        self,
        domain_id: int = 1,
        interface: str = "lo",
        gait_freq: float = 1.2,
        amp_thigh: float = 0.2,
        amp_calf: float = -0.3,
        kp: float = 50.0,
        kd: float = 3.0,
    ) -> None:
        self._domain_id = domain_id
        self._interface = interface
        self._gait_freq = gait_freq
        self._amp_thigh = amp_thigh
        self._amp_calf = amp_calf
        self._kp = kp
        self._kd = kd
        self._connected = False
        self._pub: Optional[ChannelPublisher] = None
        self._cmd = None
        self._crc = CRC()
        self._move_start_time = 0.0
        self._is_moving = False
        self._target_vx = 0.0
        self._target_vy = 0.0
        self._target_vyaw = 0.0
        self._last_cmd_time = time.time()

    def connect(self) -> bool:
        try:
            ChannelFactoryInitialize(self._domain_id, self._interface)
        except Exception:
            pass

        self._pub = ChannelPublisher("rt/lowcmd", LowCmd_)
        self._pub.Init()

        self._cmd = unitree_go_msg_dds__LowCmd_()
        self._cmd.head[0] = 0xFE
        self._cmd.head[1] = 0xEF
        self._cmd.level_flag = 0xFF
        self._cmd.gpio = 0
        for i in range(MOTOR_NUM):
            self._cmd.motor_cmd[i].mode = 0x01
            self._cmd.motor_cmd[i].q = 0.0
            self._cmd.motor_cmd[i].dq = 0.0
            self._cmd.motor_cmd[i].kp = 0.0
            self._cmd.motor_cmd[i].kd = 0.0
            self._cmd.motor_cmd[i].tau = 0.0

        self._send_stand_pose(transition_s=1.0)
        self._connected = True
        return True

    def _send_stand_pose(self, transition_s: float = 1.0) -> None:
        if self._cmd is None:
            return
        steps = int(transition_s / 0.002)
        for s in range(steps):
            phase = s / steps
            for i in range(12):
                self._cmd.motor_cmd[i].q = float(STAND_UP_JPOS[i])
                self._cmd.motor_cmd[i].kp = self._kp * (0.5 + 0.5 * phase)
                self._cmd.motor_cmd[i].kd = self._kd
                self._cmd.motor_cmd[i].mode = 0x01
            self._cmd.crc = self._crc.Crc(self._cmd)
            self._pub.Write(self._cmd)
            time.sleep(0.002)

    def move(self, vx: float, vy: float, vyaw: float) -> None:
        self._target_vx = vx
        self._target_vy = vy
        self._target_vyaw = vyaw
        self._move_start_time = time.time()
        self._is_moving = True
        self._gait_tick()

    def stand_up(self) -> None:
        self._is_moving = False
        self._send_stand_pose()

    def damp(self) -> None:
        self._is_moving = False
        if self._cmd is None:
            return
        for _ in range(50):
            for i in range(12):
                self._cmd.motor_cmd[i].q = float(STAND_UP_JPOS[i])
                self._cmd.motor_cmd[i].dq = 0.0
                self._cmd.motor_cmd[i].kp = 80.0
                self._cmd.motor_cmd[i].kd = 5.0
                self._cmd.motor_cmd[i].tau = 0.0
                self._cmd.motor_cmd[i].mode = 0x01
            self._cmd.crc = self._crc.Crc(self._cmd)
            self._pub.Write(self._cmd)
            time.sleep(0.002)
        # Then set kd max to stop
        for _ in range(50):
            for i in range(12):
                self._cmd.motor_cmd[i].kp = 0.0
                self._cmd.motor_cmd[i].kd = 10.0
                self._cmd.motor_cmd[i].dq = 0.0
                self._cmd.motor_cmd[i].tau = 0.0
            self._cmd.crc = self._crc.Crc(self._cmd)
            self._pub.Write(self._cmd)
            time.sleep(0.002)

    def recovery_stand(self) -> None:
        self.stand_up()

    def get_pose(self) -> tuple[float, float, float]:
        return (0.0, 0.0, 0.0)

    def disconnect(self) -> None:
        self._connected = False
        if self._pub:
            self._pub.Close()

    def is_connected(self) -> bool:
        return self._connected

    def _gait_tick(self) -> None:
        """Run a single iteration of the gait controller.
        Should be called at ~500Hz during movement.
        """
        if self._cmd is None or not self._is_moving:
            return

        t = time.time() - self._move_start_time
        ramp = min(t / 2.0, 1.0)
        amp_thigh = self._amp_thigh * ramp
        amp_calf = self._amp_calf * ramp

        speed_scale = min(
            math.sqrt(self._target_vx ** 2 + self._target_vy ** 2) / 0.3,
            1.0,
        ) if self._target_vx != 0 or self._target_vy != 0 else 1.0
        amp_thigh *= max(speed_scale, 0.3)
        amp_calf *= max(speed_scale, 0.3)

        for i in range(12):
            leg_idx = i // JOINTS_PER_LEG
            joint_idx = i % JOINTS_PER_LEG
            q = float(STAND_UP_JPOS[i])
            phi = 2.0 * math.pi * self._gait_freq * t + TROT_PHASES[leg_idx]

            if joint_idx == 1:  # thigh
                q += amp_thigh * math.sin(phi)
            elif joint_idx == 2:  # calf
                q += amp_calf * math.sin(phi)

            self._cmd.motor_cmd[i].q = q
            self._cmd.motor_cmd[i].dq = 0.0
            self._cmd.motor_cmd[i].kp = self._kp
            self._cmd.motor_cmd[i].kd = self._kd
            self._cmd.motor_cmd[i].tau = 0.0
            self._cmd.motor_cmd[i].mode = 0x01

        self._cmd.crc = self._crc.Crc(self._cmd)
        self._pub.Write(self._cmd)
        self._last_cmd_time = time.time()
