//! HTTP/WS gateway plus ROS bridge.
//!
//! Runs the `gateway` crate's axum server and forwards device-agnostic tasks to
//! the orchestrator's `/orchestrator/device_task` action. Inbound HTTP is served
//! by tokio; the ROS executor runs on a dedicated OS thread and the two sides
//! communicate through channels and the gateway's WebSocket event hub.

use std::net::SocketAddr;
use std::sync::{mpsc, Arc};

use anyhow::Result;
use futures::StreamExt;
use rclrs::*;

use gateway::app::{build_router, AppState};
use gateway::bridge::{BridgeCommand, RosBridge};
use gateway::config::Config;
use gateway::model::task::{Goal, TaskUpdate};
use gateway::store::task_store::TaskStore;
use task_interfaces::action::DeviceTask;

struct ChannelBridge {
    tx: mpsc::Sender<BridgeCommand>,
}

impl RosBridge for ChannelBridge {
    fn send_goal(&self, goal_id: &str, goal: Goal) {
        let _ = self.tx.send(BridgeCommand::SendGoal {
            goal_id: goal_id.to_string(),
            goal: Box::new(goal),
        });
    }

    fn cancel(&self, goal_id: &str) {
        let _ = self.tx.send(BridgeCommand::Cancel {
            goal_id: goal_id.to_string(),
        });
    }

    fn trigger_diagnosis(&self, diagnosis_id: &str) {
        let _ = self.tx.send(BridgeCommand::TriggerDiagnosis {
            diagnosis_id: diagnosis_id.to_string(),
        });
    }
}

fn to_device_task_goal(goal_id: &str, goal: &Goal) -> task_interfaces::action::DeviceTask_Goal {
    let mut g = task_interfaces::action::DeviceTask_Goal::default();
    g.goal_id = goal_id.to_string();
    g.device_id = if goal.device_id.is_empty() {
        "mock".to_string()
    } else {
        goal.device_id.clone()
    };
    g.primitive = if goal.primitive.is_empty() {
        "hold".to_string()
    } else {
        goal.primitive.clone()
    };
    if let Some(t) = &goal.target {
        g.target.kind = t.kind.clone();
        g.target.tag_id = t.tag_id;
        g.target.waypoint_id = t.waypoint_id.clone();
        g.target.action_id = t.action_id.clone();
        g.target.position_tolerance_m = t.position_tolerance_m;
        g.target.yaw_tolerance_rad = t.yaw_tolerance_rad;
    }
    g.params_json = goal.params_json.clone();
    g.constraints.max_speed_mps = goal.constraints.max_speed_mps;
    g.constraints.min_clearance_m = goal.constraints.min_clearance_m;
    g.constraints.avoid_tags = goal.constraints.avoid_tags.clone();
    g.deadline_ms = goal.deadline_ms;
    g
}

fn ros_main(
    cmd_rx: mpsc::Receiver<BridgeCommand>,
    hub: gateway::api::ws::EventHub,
    tasks: Arc<TaskStore>,
) -> Result<()> {
    let context = Context::default_from_env()?;
    let mut executor = context.create_basic_executor();
    let node = executor.create_node("gateway_bridge")?;

    let client = node.create_action_client::<DeviceTask>(ActionClientOptions::new(
        "/orchestrator/device_task",
    ))?;
    let trigger_pub = node.create_publisher::<std_msgs::msg::String>("/diagnosis/trigger")?;
    let commands = Arc::clone(node.commands());

    std::thread::spawn(move || {
        while let Ok(command) = cmd_rx.recv() {
            match command {
                BridgeCommand::SendGoal { goal_id, goal } => {
                    let client = client.clone();
                    let hub = hub.clone();
                    let tasks = Arc::clone(&tasks);
                    let forward = to_device_task_goal(&goal_id, goal.as_ref());
                    let _ = commands.run(async move {
                        let Some(goal_client) = client.request_goal(forward).await else {
                            let _ = tasks.update(
                                &goal_id,
                                &TaskUpdate {
                                    state: Some("failed".into()),
                                    error_code: Some("REJECTED".into()),
                                    final_state: Some("failed".into()),
                                    ..Default::default()
                                },
                            );
                            hub.broadcast(gateway::api::ws::Event::task(
                                &goal_id,
                                "result",
                                serde_json::json!({"final_state": "failed", "error_code": "REJECTED"}),
                            ));
                            return;
                        };
                        let mut stream = goal_client.stream();
                        while let Some(event) = stream.next().await {
                            match event {
                                GoalEvent::Feedback(fb) => {
                                    hub.broadcast(gateway::api::ws::Event::task(
                                        &goal_id,
                                        "feedback",
                                        serde_json::json!({
                                            "state": "running",
                                            "progress": fb.progress,
                                            "phase": fb.phase,
                                        }),
                                    ));
                                }
                                GoalEvent::Status(_) => {}
                                GoalEvent::Result((_status, result)) => {
                                    let final_state = if result.final_state.is_empty() {
                                        "failed".to_string()
                                    } else {
                                        result.final_state.clone()
                                    };
                                    let _ = tasks.update(
                                        &goal_id,
                                        &TaskUpdate {
                                            state: Some(final_state.clone()),
                                            error_code: Some(result.error_code.clone()),
                                            message: Some(result.message.clone()),
                                            final_state: Some(final_state.clone()),
                                            ..Default::default()
                                        },
                                    );
                                    hub.broadcast(gateway::api::ws::Event::task(
                                        &goal_id,
                                        "result",
                                        serde_json::json!({
                                            "final_state": final_state,
                                            "error_code": result.error_code,
                                            "message": result.message,
                                        }),
                                    ));
                                    break;
                                }
                            }
                        }
                    });
                }
                BridgeCommand::Cancel { goal_id } => {
                    eprintln!("[gateway_bridge] cancel not yet forwarded for {goal_id}");
                }
                BridgeCommand::TriggerDiagnosis { diagnosis_id } => {
                    let trigger_pub = trigger_pub.clone();
                    let _ = commands.run(async move {
                        let mut msg = std_msgs::msg::String::default();
                        msg.data = diagnosis_id;
                        let _ = trigger_pub.publish(&msg);
                    });
                }
            }
        }
    });

    executor.spin(SpinOptions::default()).first_error()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Config::from_env();
    let (tx, rx) = mpsc::channel();
    let bridge = Arc::new(ChannelBridge { tx });
    let state = AppState::new(config, bridge)?;

    {
        let hub = state.hub.clone();
        let tasks = state.tasks.clone();
        std::thread::spawn(move || {
            if let Err(err) = ros_main(rx, hub, tasks) {
                eprintln!("[gateway_bridge] ros error: {err}");
            }
        });
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], state.config.http_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("gateway_bridge listening on http://{addr}");
    axum::serve(listener, build_router(state)).await?;
    Ok(())
}
