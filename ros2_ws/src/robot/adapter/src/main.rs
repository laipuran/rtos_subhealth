//! Parameterized device adapter node.
//!
//! Hosts a `DeviceTask` action server at `/<device_id>/device_task`, drives a
//! `device_sdk::DeviceBackend` (mock or differential-drive simulation), and
//! publishes the device's `DeviceDescriptor` (transient-local) and
//! `DeviceState` so the orchestrator can discover it.
//!
//! Configuration (environment, set by the launch/unit file):
//!   DEVICE_ID    device identifier (default "mock")
//!   DEVICE_TYPE  mock | diff_drive (default "mock")

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Result};
use futures_timer::Delay;
use rclrs::*;

use device_interfaces::msg::{DeviceDescriptor, DeviceState};
use device_sdk::{
    BackendStatus, Command, DeviceBackend, DiffDriveSim, MockDevice, Pose2d, Velocity,
};
use orchestrator_core::Primitive;
use task_interfaces::action::{DeviceTask, DeviceTask_Feedback, DeviceTask_Goal, DeviceTask_Result};

fn quat_to_yaw(q: &geometry_msgs::msg::Quaternion) -> f64 {
    let (x, y, z, w) = (q.x as f64, q.y as f64, q.z as f64, q.w as f64);
    (2.0 * (w * z + x * y)).atan2(1.0 - 2.0 * (y * y + z * z))
}

fn to_command(goal: &DeviceTask_Goal) -> Result<Command> {
    match goal.primitive.as_str() {
        "move_to_pose" => {
            let p = &goal.target.pose.pose.position;
            let yaw = quat_to_yaw(&goal.target.pose.pose.orientation);
            let pos_tol = if goal.target.position_tolerance_m > 0.0 {
                goal.target.position_tolerance_m as f64
            } else {
                0.1
            };
            let yaw_tol = if goal.target.yaw_tolerance_rad > 0.0 {
                goal.target.yaw_tolerance_rad as f64
            } else {
                0.2
            };
            Ok(Command::MoveToPose {
                target: Pose2d::new(p.x as f64, p.y as f64, yaw),
                position_tolerance_m: pos_tol,
                yaw_tolerance_rad: yaw_tol,
            })
        }
        "set_velocity" => {
            let v: serde_json::Value = serde_json::from_str(&goal.params_json).unwrap_or_default();
            Ok(Command::SetVelocity {
                velocity: Velocity::new(
                    v["vx"].as_f64().unwrap_or(0.0),
                    v["vy"].as_f64().unwrap_or(0.0),
                    v["vyaw"].as_f64().unwrap_or(0.0),
                ),
                duration_s: v["duration_s"].as_f64().unwrap_or(0.0),
            })
        }
        "hold" => {
            let v: serde_json::Value = serde_json::from_str(&goal.params_json).unwrap_or_default();
            Ok(Command::Hold {
                duration_s: v["duration_s"].as_f64().unwrap_or(0.0),
            })
        }
        "stop" => Ok(Command::Stop),
        "execute_primitive" => {
            let params: serde_json::Value =
                serde_json::from_str(&goal.params_json).unwrap_or_default();
            let name = params["action"]
                .as_str()
                .unwrap_or(goal.target.action_id.as_str())
                .to_string();
            Ok(Command::ExecutePrimitive { name, params })
        }
        other => Err(anyhow!("unknown primitive: {other}")),
    }
}

fn make_backend(device_id: &str, device_type: &str) -> Box<dyn DeviceBackend> {
    match device_type {
        "diff_drive" => Box::new(DiffDriveSim::new(device_id)),
        _ => Box::new(MockDevice::new(
            device_id,
            &[
                Primitive::ExecutePrimitive,
                Primitive::Hold,
                Primitive::Stop,
            ],
            5,
        )),
    }
}

fn to_descriptor_msg(d: &orchestrator_core::DeviceDescriptor) -> DeviceDescriptor {
    let mut msg = DeviceDescriptor::default();
    msg.device_id = d.device_id.clone();
    msg.name = d.name.clone();
    msg.kind = d.kind.as_str().to_string();
    msg.capabilities = d.capabilities.clone();
    msg.primitives = d.primitives.clone();
    msg.limits.max_speed_mps = d.limits.max_speed_mps;
    msg.limits.max_yaw_rate_rps = d.limits.max_yaw_rate_rps;
    msg.limits.max_slope_rad = d.limits.max_slope_rad;
    msg.limits.position_tolerance_m = d.limits.position_tolerance_m;
    msg.limits.yaw_tolerance_rad = d.limits.yaw_tolerance_rad;
    msg.frames = d.frames.clone();
    msg.sensors = d.sensors.clone();
    msg
}

fn main() -> Result<()> {
    let context = Context::default_from_env()?;
    let mut executor = context.create_basic_executor();
    let node = executor.create_node("adapter")?;

    let device_id = std::env::var("DEVICE_ID").unwrap_or_else(|_| "mock".into());
    let device_type = std::env::var("DEVICE_TYPE").unwrap_or_else(|_| "mock".into());

    let backend: Arc<Mutex<Box<dyn DeviceBackend>>> =
        Arc::new(Mutex::new(make_backend(&device_id, &device_type)));

    // Descriptor is latched so late-joining orchestrators discover the device.
    let desc_pub = node.create_publisher::<DeviceDescriptor>(
        "/device_descriptors".keep_last(1).transient_local(),
    )?;
    let descriptor = to_descriptor_msg(backend.lock().unwrap().descriptor());
    desc_pub.publish(&descriptor)?;

    // Periodic state.
    let state_pub = node.create_publisher::<DeviceState>("/device_states")?;
    let state_backend = Arc::clone(&backend);
    let _state_timer = node.create_timer_repeating(Duration::from_millis(100), move || {
        let b = state_backend.lock().unwrap();
        let mut msg = DeviceState::default();
        msg.device_id = b.descriptor().device_id.clone();
        msg.pose.header.frame_id = "map".into();
        if let Some(p) = b.pose() {
            msg.pose.pose.position.x = p.x;
            msg.pose.pose.position.y = p.y;
            msg.pose.pose.orientation.z = (p.yaw / 2.0).sin();
            msg.pose.pose.orientation.w = (p.yaw / 2.0).cos();
        }
        msg.healthy = true;
        let _ = state_pub.publish(&msg);
    })?;

    let action_name = format!("/{device_id}/device_task");
    let action_backend = Arc::clone(&backend);
    let _server = node.create_action_server::<DeviceTask, _>(
        ActionServerOptions::new(action_name.as_str()),
        move |requested| {
            let action_backend = Arc::clone(&action_backend);
            async move {
                let goal = requested.goal().clone();
                let command = match to_command(goal.as_ref()) {
                    Ok(c) => c,
                    Err(_) => return requested.reject(),
                };
                if action_backend.lock().unwrap().command(command).is_err() {
                    return requested.reject();
                }

                let executing = requested.accept().execute();
                let mut feedback = DeviceTask_Feedback::default();
                feedback.state = "running".into();
                loop {
                    let status = action_backend.lock().unwrap().step(0.1);
                    match status {
                        BackendStatus::Running { progress } => {
                            feedback.progress = progress;
                            executing.publish_feedback(feedback.clone());
                            Delay::new(Duration::from_millis(100)).await;
                        }
                        BackendStatus::Succeeded => {
                            let mut result = DeviceTask_Result::default();
                            result.final_state = "succeeded".into();
                            return executing.succeeded_with(result);
                        }
                        BackendStatus::Failed { code, message } => {
                            let mut result = DeviceTask_Result::default();
                            result.final_state = "failed".into();
                            result.error_code = code;
                            result.message = message;
                            return executing.aborted_with(result);
                        }
                    }
                }
            }
        },
    )?;

    executor.spin(SpinOptions::default()).first_error()?;
    Ok(())
}
