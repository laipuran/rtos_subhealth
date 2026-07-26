"""极简验证：直接给 mj_data.ctrl 加一个大的阶跃信号，看关节动不动"""
import sys, os, time, threading
sys.path.insert(0, os.path.expanduser("~/unitree_sdk2_python"))
sys.path.insert(0, os.path.expanduser("~/unitree_mujoco/simulate_python"))
import mujoco, mujoco.viewer
import config

scene_abs = os.path.expanduser(f"~/unitree_mujoco/unitree_robots/{config.ROBOT}/scene.xml")
print(f"Loading: {scene_abs}")
mj_model = mujoco.MjModel.from_xml_path(scene_abs)
mj_data = mujoco.MjData(mj_model)

print(f"  nu (actuator count) = {mj_model.nu}")
print(f"  nsensor = {mj_model.nsensor}")
print(f"  nq = {mj_model.nq}, nv = {mj_model.nv}")
print(f"  sensordata[:3] = {mj_data.sensordata[:3]}")

viewer = mujoco.viewer.launch_passive(mj_model, mj_data)
sim_running = True
locker = threading.Lock()

def sim_thread():
    global sim_running
    while viewer.is_running() and sim_running:
        with locker:
            mujoco.mj_step(mj_model, mj_data)
        time.sleep(mj_model.opt.timestep)

def view_thread():
    while viewer.is_running() and sim_running:
        with locker:
            viewer.sync()
        time.sleep(0.02)

t1 = threading.Thread(target=sim_thread, daemon=True)
t2 = threading.Thread(target=view_thread, daemon=True)
t1.start(); t2.start()
time.sleep(1)

print("\n=== 阶段1: ctrl = 0 (静止)，观察 2 秒 ===")
for _ in range(200):
    with locker:
        for i in range(mj_model.nu):
            mj_data.ctrl[i] = 0.0
    time.sleep(0.01)
print(f"  sensordata[:3] = {mj_data.sensordata[:3]}")

print("\n=== 阶段2: ctrl = 10 (大扭矩)，观察 3 秒 ===")
for _ in range(300):
    with locker:
        for i in range(mj_model.nu):
            mj_data.ctrl[i] = 10.0
    time.sleep(0.01)
print(f"  sensordata[:3] = {mj_data.sensordata[:3]}")

print("\n=== 阶段3: ctrl = 0 (放松)，观察 2 秒 ===")
for _ in range(200):
    with locker:
        for i in range(mj_model.nu):
            mj_data.ctrl[i] = 0.0
    time.sleep(0.01)
print(f"  sensordata[:3] = {mj_data.sensordata[:3]}")

sim_running = False
time.sleep(0.5)
print("\nDone")
