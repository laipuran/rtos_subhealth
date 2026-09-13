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
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use futures_timer::Delay;
use rclrs::*;

use device_interfaces::msg::{DeviceDescriptor, DeviceState};
use device_sdk::tonypi::{ActionRunner, TonyPiBackend};
use device_sdk::{
    BackendStatus, Command, DeviceBackend, DiffDriveSim, MockDevice, Pose2d, Velocity,
};
use orchestrator_core::Primitive;
use safety::{SafetyLimits, SafetySupervisor};
use task_interfaces::action::{DeviceTask, DeviceTask_Feedback, DeviceTask_Goal, DeviceTask_Result};

fn now_s() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

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

/// JSON-RPC action runner for a Hiwonder TonyPi (`:9030`, `RunAction`).
///
/// Uses a minimal blocking HTTP/1.1 POST over `std::net::TcpStream` so the
/// adapter keeps a Rust-1.85-compatible dependency set and needs no TLS (the
/// robot RPC is plain HTTP on the local network).
struct HttpActionRunner {
    host: String,
    port: u16,
    path: String,
}

impl HttpActionRunner {
    fn from_url(url: &str) -> Result<Self, String> {
        let rest = url
            .strip_prefix("http://")
            .ok_or_else(|| "only http:// URLs are supported".to_string())?;
        let (authority, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let (host, port) = match authority.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), p.parse::<u16>().map_err(|_| "bad port")?),
            None => (authority.to_string(), 80),
        };
        Ok(Self {
            host,
            port,
            path: path.to_string(),
        })
    }
}

impl ActionRunner for HttpActionRunner {
    fn run_action(&mut self, action: &str) -> Result<(), String> {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "RunAction",
            "params": [action, 1],
            "id": 1,
        })
        .to_string();
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.path,
            self.host,
            self.port,
            body.len(),
            body
        );

        let mut stream = TcpStream::connect((self.host.as_str(), self.port))
            .map_err(|e| e.to_string())?;
        stream
            .write_all(request.as_bytes())
            .map_err(|e| e.to_string())?;
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .map_err(|e| e.to_string())?;

        let status = response.lines().next().unwrap_or("");
        if status.contains(" 200 ") || status.contains(" 204 ") {
            Ok(())
        } else {
            Err(format!("RPC failed: {status}"))
        }
    }
}

fn make_backend(device_id: &str, device_type: &str) -> Box<dyn DeviceBackend> {
    match device_type {
        "diff_drive" => Box::new(DiffDriveSim::new(device_id)),
        "tonypi" => {
            let url = std::env::var("TONYPI_RPC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:9030/".into());
            match HttpActionRunner::from_url(&url) {
                Ok(runner) => Box::new(TonyPiBackend::new(device_id, runner)),
                Err(err) => {
                    eprintln!("invalid TONYPI_RPC_URL '{url}': {err}");
                    Box::new(MockDevice::new(
                        device_id,
                        &[Primitive::Stop],
                        1,
                    ))
                }
            }
        }
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

    let limits = {
        let b = backend.lock().unwrap();
        let l = &b.descriptor().limits;
        SafetyLimits {
            max_speed_mps: l.max_speed_mps as f64,
            max_yaw_rate_rps: l.max_yaw_rate_rps as f64,
        }
    };
    let watchdog = std::env::var("SAFETY_WATCHDOG_S")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5.0);
    let safety = Arc::new(Mutex::new(SafetySupervisor::new(limits, watchdog)));

    // Descriptor is latched so late-joining orchestrators discover the device.
    let desc_pub = node.create_publisher::<DeviceDescriptor>(
        "/device_descriptors".keep_last(1).transient_local(),
    )?;
    let descriptor = to_descriptor_msg(backend.lock().unwrap().descriptor());
    desc_pub.publish(&descriptor)?;

    // Periodic state.
    let state_pub = node.create_publisher::<DeviceState>("/device_states")?;
    let state_backend = Arc::clone(&backend);
    let safety_hb = Arc::clone(&safety);
    let _state_timer = node.create_timer_repeating(Duration::from_millis(100), move || {
        safety_hb.lock().unwrap().heartbeat(now_s());
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
    let action_safety = Arc::clone(&safety);
    let _server = node.create_action_server::<DeviceTask, _>(
        ActionServerOptions::new(action_name.as_str()),
        move |requested| {
            let action_backend = Arc::clone(&action_backend);
            let safety = Arc::clone(&action_safety);
            async move {
                let goal = requested.goal().clone();
                let command = match to_command(goal.as_ref()) {
                    Ok(c) => c,
                    Err(_) => return requested.reject(),
                };
                let command = match safety.lock().unwrap().sanitize(command, now_s()) {
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
