from __future__ import annotations

import json
import math
import os
import threading
import time
from typing import Optional

import numpy as np

from .interface import RobotInterface

NUM_MOTOR = 12
JOINTS_PER_LEG = 3

stand_up = np.array([
    0.00571868, 0.608813, -1.21763,
    -0.00571868, 0.608813, -1.21763,
    0.00571868, 0.608813, -1.21763,
    -0.00571868, 0.608813, -1.21763,
], dtype=float)

stand_down = np.array([
    0.0473455, 1.22187, -2.44375,
    -0.0473455, 1.22187, -2.44375,
    0.0473455, 1.22187, -2.44375,
    -0.0473455, 1.22187, -2.44375,
], dtype=float)

TROT_PHASES = [0.0, math.pi, math.pi, 0.0]


class SimRobot(RobotInterface):
    """In-process MuJoCo simulation + gait control.

    No DDS dependency. Runs MuJoCo in background threads.
    """

    def __init__(
        self,
        maps_path: str = "",
        scene_path: str = "",
        gait_freq: float = 1.2,
        amp_thigh: float = 0.2,
        amp_calf: float = -0.3,
        kp: float = 50.0,
        kd: float = 3.0,
    ) -> None:
        self._maps_path = maps_path
        self._scene_path = scene_path
        self._gait_freq = gait_freq
        self._amp_thigh = amp_thigh
        self._amp_calf = amp_calf
        self._kp = kp
        self._kd = kd
        self._connected = False
        self._sim_running = False
        self._mj_model = None
        self._mj_data = None
        self._viewer = None
        self._locker = threading.Lock()
        self._tags: dict[str, dict] = {}
        self._robot_x = 0.0
        self._robot_y = 0.0

    def connect(self) -> bool:
        if self._connected:
            return True

        scene = self._scene_path
        if not scene:
            scene = os.path.expanduser("~/unitree_mujoco/unitree_robots/go2/scene.xml")
            # Also try via config
            try:
                sys.path.insert(0, os.path.expanduser("~/unitree_mujoco/simulate_python"))
                import config
                scene = os.path.expanduser(f"~/unitree_mujoco/unitree_robots/{config.ROBOT}/scene.xml")
            except Exception:
                pass

        if not os.path.exists(scene):
            return False

        import mujoco
        import mujoco.viewer

        self._mj_model = mujoco.MjModel.from_xml_path(scene)
        self._mj_data = mujoco.MjData(self._mj_model)

        # Set timestep
        try:
            import config
            self._mj_model.opt.timestep = config.SIMULATE_DT
        except Exception:
            self._mj_model.opt.timestep = 0.005

        self._viewer = mujoco.viewer.launch_passive(self._mj_model, self._mj_data)
        self._sim_running = True

        # Start sim + viewer threads
        t_sim = threading.Thread(target=self._sim_loop, daemon=True)
        t_view = threading.Thread(target=self._view_loop, daemon=True)
        t_sim.start()
        t_view.start()

        # Load tag map
        self._load_map()

        self._connected = True
        return True

    def _sim_loop(self) -> None:
        while self._sim_running and self._viewer.is_running():
            step_start = time.perf_counter()
            with self._locker:
                import mujoco
                mujoco.mj_step(self._mj_model, self._mj_data)
            time.sleep(max(0, self._mj_model.opt.timestep -
                           (time.perf_counter() - step_start)))

    def _view_loop(self) -> None:
        while self._sim_running and self._viewer.is_running():
            with self._locker:
                self._viewer.sync()
            time.sleep(0.02)

    def _load_map(self) -> None:
        path = os.path.join(self._maps_path, "default.json")
        if not os.path.exists(path):
            return
        with open(path) as f:
            data = json.load(f)
        self._tags = {str(k): v for k, v in data.get("tags", {}).items()}

    def move(self, vx: float, vy: float, vyaw: float) -> None:
        pass

    def move_blocking(self, vx: float, vy: float, vyaw: float,
                       duration: float) -> None:
        t0 = time.time()
        while time.time() - t0 < duration:
            if not self._sim_running:
                break
            t = time.time() - t0
            ramp = min(t / 2.0, 1.0)
            with self._locker:
                for i in range(NUM_MOTOR):
                    leg = i // JOINTS_PER_LEG
                    joint = i % JOINTS_PER_LEG
                    q_des = stand_up[i]
                    phi = 2.0 * math.pi * self._gait_freq * t + TROT_PHASES[leg]
                    if joint == 1:
                        q_des += self._amp_thigh * ramp * math.sin(phi)
                    elif joint == 2:
                        q_des += self._amp_calf * ramp * math.sin(phi)
                    self._mj_data.ctrl[i] = (
                        self._kp * (q_des - self._mj_data.sensordata[i])
                        - self._kd * self._mj_data.sensordata[i + NUM_MOTOR]
                    )
            time.sleep(0.002)

    def stand_up(self) -> None:
        t0 = time.time()
        while time.time() - t0 < 2.0:
            t = time.time() - t0
            phase = np.tanh(t / 1.2)
            kp_now = 20.0 + phase * 30.0
            with self._locker:
                for i in range(NUM_MOTOR):
                    q_des = phase * stand_up[i] + (1 - phase) * stand_down[i]
                    self._mj_data.ctrl[i] = (
                        kp_now * (q_des - self._mj_data.sensordata[i])
                        - self._kd * self._mj_data.sensordata[i + NUM_MOTOR]
                    )
            time.sleep(0.005)

    def damp(self) -> None:
        with self._locker:
            for i in range(NUM_MOTOR):
                q_des = stand_up[i]
                self._mj_data.ctrl[i] = (
                    80.0 * (q_des - self._mj_data.sensordata[i])
                    - 5.0 * self._mj_data.sensordata[i + NUM_MOTOR]
                )

    def recovery_stand(self) -> None:
        self.stand_up()

    def get_pose(self) -> tuple[float, float, float]:
        with self._locker:
            if self._mj_data is None:
                return (0.0, 0.0, 0.0)
            sensordata = self._mj_data.sensordata
            idx = 3 * NUM_MOTOR
            x = float(sensordata[idx]) if len(sensordata) > idx else 0.0
            y = float(sensordata[idx + 1]) if len(sensordata) > idx + 1 else 0.0
            return (x, y, 0.0)

    def check_tag_arrival(self, target_tag: int) -> bool:
        px, py, _ = self.get_pose()
        tag_key = str(target_tag)
        if tag_key not in self._tags:
            return False
        tx = self._tags[tag_key].get("x", 0.0)
        ty = self._tags[tag_key].get("y", 0.0)
        dist = math.sqrt((px - tx) ** 2 + (py - ty) ** 2)
        return dist < 0.5

    def disconnect(self) -> None:
        self._sim_running = False
        self._connected = False

    def is_connected(self) -> bool:
        return self._connected
