//! HTTP/WS gateway plus ROS bridge.
//!
//! Runs the `gateway` crate's axum server and forwards device-agnostic tasks to
//! the orchestrator's `/orchestrator/device_task` action. Inbound HTTP is served
//! by tokio; the ROS executor runs on a dedicated OS thread and the two sides
//! communicate through channels and the gateway's WebSocket event hub.

use std::net::SocketAddr;
use std::sync::{mpsc, Arc};

use anyhow::Result;
use diagnosis_interfaces::msg::{DiagnosisMetric, DiagnosisResult, VitalsStream};
use futures::StreamExt;
use rclrs::*;
use serde_json::{json, Value};

use gateway::app::{build_router, AppState};
use gateway::bridge::{BridgeCommand, RosBridge};
use gateway::config::Config;
use gateway::model::task::{Goal, TaskUpdate};
use gateway::store::diagnosis_store::{DiagnosisRecord, DiagnosisStore};
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

fn timestamp_seconds(sec: i32, nanosec: u32) -> f64 {
    sec as f64 + nanosec as f64 / 1_000_000_000.0
}

fn metric_payload(metric: DiagnosisMetric) -> Value {
    json!({
        "data_src": metric.data_src,
        "data_type": metric.data_type,
        "latest": metric.latest,
        "mean": metric.mean,
        "min": metric.min,
        "max": metric.max,
        "trend": metric.trend,
        "valid": metric.valid,
    })
}

fn diagnosis_record_from_message(message: DiagnosisResult) -> DiagnosisRecord {
    let created_at = if message.timestamp.sec == 0 && message.timestamp.nanosec == 0 {
        gateway::store::now()
    } else {
        timestamp_seconds(message.timestamp.sec, message.timestamp.nanosec)
    };

    DiagnosisRecord {
        diagnosis_id: message.diagnosis_id,
        source_ids: message.source_ids,
        trigger_type: message.trigger_type,
        severity: message.severity,
        summary: message.summary,
        possible_causes: message.possible_causes,
        recommendations: message.recommendations,
        confidence: message.confidence as f64,
        disclaimer: message.disclaimer,
        raw_prompt: message.raw_prompt,
        error_code: message.error_code,
        error_message: message.error_message,
        metrics: message.metrics.into_iter().map(metric_payload).collect(),
        created_at,
    }
}

fn vitals_payload_from_message(message: VitalsStream) -> Value {
    json!({
        "timestamp": timestamp_seconds(message.timestamp.sec, message.timestamp.nanosec),
        "metrics": message.metrics.into_iter().map(metric_payload).collect::<Vec<_>>(),
    })
}

fn ros_main(
    cmd_rx: mpsc::Receiver<BridgeCommand>,
    hub: gateway::api::ws::EventHub,
    tasks: Arc<TaskStore>,
    diagnoses: Arc<DiagnosisStore>,
) -> Result<()> {
    let context = Context::default_from_env()?;
    let mut executor = context.create_basic_executor();
    let node = executor.create_node("gateway_bridge")?;

    let client = node.create_action_client::<DeviceTask>(ActionClientOptions::new(
        "/orchestrator/device_task",
    ))?;
    let trigger_pub = node.create_publisher::<std_msgs::msg::String>("/diagnosis/trigger")?;
    let commands = Arc::clone(node.commands());

    let diagnosis_hub = hub.clone();
    let _diagnosis_subscription = node.create_subscription::<DiagnosisResult, _>(
        "/diagnosis/results",
        move |message: DiagnosisResult| {
            let record = diagnosis_record_from_message(message);
            if diagnoses.add(&record).is_ok() {
                diagnosis_hub.broadcast(gateway::api::ws::Event::diagnosis(record.to_wire_dict()));
            }
        },
    )?;
    let vitals_hub = hub.clone();
    let _vitals_subscription = node.create_subscription::<VitalsStream, _>(
        "/diagnosis/monitor",
        move |message: VitalsStream| {
            vitals_hub.broadcast(gateway::api::ws::Event::vitals(
                vitals_payload_from_message(message),
            ));
        },
    )?;

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
        let diagnoses = state.diagnoses.clone();
        std::thread::spawn(move || {
            if let Err(err) = ros_main(rx, hub, tasks, diagnoses) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn metric() -> diagnosis_interfaces::msg::DiagnosisMetric {
        let mut metric = diagnosis_interfaces::msg::DiagnosisMetric::default();
        metric.data_src = "mock_spo2".into();
        metric.data_type = "spo2".into();
        metric.latest = 97.0;
        metric.mean = 96.5;
        metric.min = 95.0;
        metric.max = 98.0;
        metric.trend = "stable".into();
        metric.valid = true;
        metric
    }

    #[test]
    fn diagnosis_projection_preserves_message_fields_and_ros_timestamp() {
        let mut message = diagnosis_interfaces::msg::DiagnosisResult::default();
        message.timestamp.sec = 123;
        message.timestamp.nanosec = 456;
        message.diagnosis_id = "diag-1".into();
        message.source_ids = vec!["mock_spo2".into()];
        message.trigger_type = "anomaly".into();
        message.severity = "moderate".into();
        message.summary = "oxygen saturation below threshold".into();
        message.possible_causes = vec!["sensor noise".into()];
        message.recommendations = vec!["repeat measurement".into()];
        message.confidence = 0.75;
        message.disclaimer = "not medical advice".into();
        message.raw_prompt = "raw input".into();
        message.error_code = "".into();
        message.error_message = "".into();
        message.metrics = vec![metric()];

        let record = diagnosis_record_from_message(message);

        assert_eq!(record.diagnosis_id, "diag-1");
        assert_eq!(record.source_ids, vec!["mock_spo2"]);
        assert_eq!(record.trigger_type, "anomaly");
        assert_eq!(record.severity, "moderate");
        assert_eq!(record.summary, "oxygen saturation below threshold");
        assert_eq!(record.possible_causes, vec!["sensor noise"]);
        assert_eq!(record.recommendations, vec!["repeat measurement"]);
        assert_eq!(record.confidence, 0.75);
        assert_eq!(record.disclaimer, "not medical advice");
        assert_eq!(record.raw_prompt, "raw input");
        assert_eq!(record.error_code, "");
        assert_eq!(record.error_message, "");
        assert_eq!(record.created_at, 123.000000456);
        assert_eq!(
            record.metrics,
            vec![serde_json::json!({
                "data_src": "mock_spo2",
                "data_type": "spo2",
                "latest": 97.0,
                "mean": 96.5,
                "min": 95.0,
                "max": 98.0,
                "trend": "stable",
                "valid": true,
            })]
        );
    }

    #[test]
    fn diagnosis_projection_uses_receipt_time_for_zero_ros_timestamp() {
        let before = gateway::store::now();
        let record =
            diagnosis_record_from_message(diagnosis_interfaces::msg::DiagnosisResult::default());
        let after = gateway::store::now();

        assert!(record.created_at >= before);
        assert!(record.created_at <= after);
    }

    #[test]
    fn vitals_projection_contains_timestamp_and_metrics() {
        let mut message = diagnosis_interfaces::msg::VitalsStream::default();
        message.timestamp.sec = 42;
        message.timestamp.nanosec = 500_000_000;
        message.metrics = vec![metric()];

        assert_eq!(
            vitals_payload_from_message(message),
            serde_json::json!({
                "timestamp": 42.5,
                "metrics": [{
                    "data_src": "mock_spo2",
                    "data_type": "spo2",
                    "latest": 97.0,
                    "mean": 96.5,
                    "min": 95.0,
                    "max": 98.0,
                    "trend": "stable",
                    "valid": true,
                }],
            })
        );
    }
}
