"""单进程 MuJoCo 行走测试（无 DDS bridge，避免线程崩溃）"""
import sys, os, math, time, threading
import numpy as np
sys.path.insert(0, os.path.expanduser("~/unitree_mujoco/simulate_python"))
import mujoco, mujoco.viewer
import config

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

scene_abs = os.path.expanduser(f"~/unitree_mujoco/unitree_robots/{config.ROBOT}/scene.xml")
print(f"Loading: {scene_abs}")
mj_model = mujoco.MjModel.from_xml_path(scene_abs)
mj_data = mujoco.MjData(mj_model)
mj_model.opt.timestep = config.SIMULATE_DT

viewer = mujoco.viewer.launch_passive(mj_model, mj_data)
sim_running = True
locker = threading.Lock()

def sim_thread():
    global sim_running
    while viewer.is_running() and sim_running:
        step_start = time.perf_counter()
        with locker:
            mujoco.mj_step(mj_model, mj_data)
        time.sleep(max(0, mj_model.opt.timestep - (time.perf_counter() - step_start)))

def view_thread():
    while viewer.is_running() and sim_running:
        with locker:
            viewer.sync()
        time.sleep(config.VIEWER_DT)

t_sim = threading.Thread(target=sim_thread, daemon=True)
t_view = threading.Thread(target=view_thread, daemon=True)
t_sim.start()
t_view.start()
time.sleep(0.5)

freq = 1.2
amp_thigh = 0.2
amp_calf = -0.3
k_p_max = 50.0
kd = 3.0

# 起立：从 stand_down 平滑过渡到 stand_up
print("Standing up...")
t0 = time.time()
while time.time() - t0 < 2.0:
    t = time.time() - t0
    phase = np.tanh(t / 1.2)
    kp = 20.0 + phase * 30.0
    with locker:
        for i in range(NUM_MOTOR):
            q_des = phase * stand_up[i] + (1 - phase) * stand_down[i]
            mj_data.ctrl[i] = kp * (q_des - mj_data.sensordata[i]) - kd * mj_data.sensordata[i + NUM_MOTOR]
    time.sleep(0.005)

print("Walking (10s)...")
t0 = time.time()
while time.time() - t0 < 10.0:
    t = time.time() - t0
    ramp = min(t / 2.0, 1.0)
    with locker:
        for i in range(NUM_MOTOR):
            leg = i // JOINTS_PER_LEG
            joint = i % JOINTS_PER_LEG
            q_des = stand_up[i]
            phi = 2.0 * math.pi * freq * t + TROT_PHASES[leg]
            if joint == 1:
                q_des += amp_thigh * ramp * math.sin(phi)
            elif joint == 2:
                q_des += amp_calf * ramp * math.sin(phi)
            mj_data.ctrl[i] = k_p_max * (q_des - mj_data.sensordata[i]) - kd * mj_data.sensordata[i + NUM_MOTOR]
    time.sleep(0.002)
    if int(t) > int(t - 0.003):
        print(f"  t={t:.1f}s  base_z={mj_data.sensordata[3*12+2]:.3f}  motor[1].q={mj_data.sensordata[1]:.3f}")

print("Stopping...")
for _ in range(100):
    with locker:
        for i in range(NUM_MOTOR):
            q_des = stand_up[i]
            mj_data.ctrl[i] = 80.0 * (q_des - mj_data.sensordata[i]) - 5.0 * mj_data.sensordata[i + NUM_MOTOR]
    time.sleep(0.005)

sim_running = False
time.sleep(0.5)
print("Done")
