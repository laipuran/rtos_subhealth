//! Device-agnostic orchestrator node.
//!
//! Discovers device adapters from `/device_descriptors`, validates incoming
//! `DeviceTask` goals with `orchestrator-core`, and forwards accepted goals to
//! the target adapter's `/<device_id>/device_task` action server, relaying
//! feedback and results.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use futures::channel::oneshot;
use futures::StreamExt;
use rclrs::*;

use device_interfaces::msg::DeviceDescriptor as DeviceDescriptorMsg;
use orchestrator_core::{Orchestrator, Primitive, Task, TaskOutcome, TargetKind};
use task_interfaces::action::{DeviceTask, DeviceTask_Feedback, DeviceTask_Goal, DeviceTask_Result};

fn from_descriptor_msg(m: &DeviceDescriptorMsg) -> orchestrator_core::DeviceDescriptor {
    orchestrator_core::DeviceDescriptor {
        device_id: m.device_id.clone(),
        name: m.name.clone(),
        kind: orchestrator_core::DeviceKind::parse(&m.kind),
        capabilities: m.capabilities.clone(),
        primitives: m.primitives.clone(),
        limits: orchestrator_core::DeviceLimits {
            max_speed_mps: m.limits.max_speed_mps,
            max_yaw_rate_rps: m.limits.max_yaw_rate_rps,
            max_slope_rad: m.limits.max_slope_rad,
            position_tolerance_m: m.limits.position_tolerance_m,
            yaw_tolerance_rad: m.limits.yaw_tolerance_rad,
        },
        frames: m.frames.clone(),
        sensors: m.sensors.clone(),
    }
}

fn to_target(msg: &device_interfaces::msg::TaskTarget) -> orchestrator_core::TaskTarget {
    let kind = match msg.kind.as_str() {
        "tag" => TargetKind::Tag,
        "waypoint" => TargetKind::Waypoint,
        "action" => TargetKind::Action,
        _ => TargetKind::Pose,
    };
    orchestrator_core::TaskTarget {
        kind,
        tag_id: Some(msg.tag_id),
        waypoint_id: (!msg.waypoint_id.is_empty()).then(|| msg.waypoint_id.clone()),
        action_id: (!msg.action_id.is_empty()).then(|| msg.action_id.clone()),
        position_tolerance_m: Some(msg.position_tolerance_m),
        yaw_tolerance_rad: Some(msg.yaw_tolerance_rad),
    }
}

fn failed_result(code: &str, message: &str) -> DeviceTask_Result {
    let mut r = DeviceTask_Result::default();
    r.final_state = "failed".into();
    r.error_code = code.into();
    r.message = message.into();
    r
}

fn main() -> Result<()> {
    let context = Context::default_from_env()?;
    let mut executor = context.create_basic_executor();
    let node = executor.create_node("orchestrator")?;

    let orchestrator = Arc::new(Mutex::new(Orchestrator::new(
        orchestrator_core::DeviceRegistry::new(),
    )));

    // Discover adapters from their latched descriptors.
    let reg_orch = Arc::clone(&orchestrator);
    let _descriptor_sub = node.create_subscription::<DeviceDescriptorMsg, _>(
        "/device_descriptors".keep_last(1).transient_local(),
        move |msg: DeviceDescriptorMsg| {
            let descriptor = from_descriptor_msg(&msg);
            let mut orch = reg_orch.lock().unwrap();
            match orch.registry_mut().register(descriptor) {
                Ok(()) => {}
                // A re-published descriptor is not an error.
                Err(orchestrator_core::RegistryError::Duplicate(_)) => {}
                Err(err) => eprintln!("descriptor registration failed: {err}"),
            }
        },
    )?;

    let srv_orch = Arc::clone(&orchestrator);
    let srv_node = Arc::clone(&node);
    let _server = node.create_action_server::<DeviceTask, _>(
        ActionServerOptions::new("/orchestrator/device_task"),
        move |requested| {
            let orch = Arc::clone(&srv_orch);
            let node = Arc::clone(&srv_node);
            async move {
                let goal: Arc<DeviceTask_Goal> = requested.goal().clone();

                let Some(primitive) = Primitive::parse(&goal.primitive) else {
                    return requested.reject();
                };
                let task = Task {
                    goal_id: goal.goal_id.clone(),
                    device_id: goal.device_id.clone(),
                    primitive,
                    target: Some(to_target(&goal.target)),
                    params_json: (!goal.params_json.is_empty()).then(|| goal.params_json.clone()),
                    deadline_ms: goal.deadline_ms,
                };
                if orch.lock().unwrap().submit(task).is_err() {
                    return requested.reject();
                }

                let action_name = format!("/{}/device_task", goal.device_id);
                let client = match node.create_action_client::<DeviceTask>(
                    ActionClientOptions::new(action_name.as_str()),
                ) {
                    Ok(c) => c,
                    Err(_) => {
                        orch.lock()
                            .unwrap()
                            .complete(&goal.goal_id, TaskOutcome::failed("INTERNAL", "client create failed"));
                        return requested.reject();
                    }
                };

                let accepted = requested.accept();
                let feedback_pub = accepted.feedback_publisher();
                let (tx, rx) = oneshot::channel::<DeviceTask_Result>();

                // The client stream is Send but not Sync, so it cannot live in
                // the (Sync) server future. Run the forwarding on the executor
                // and bridge the result back through a oneshot channel.
                let forward_goal = Arc::clone(&goal);
                let _promise = node.commands().run(async move {
                    let goal_client = match client.request_goal((*forward_goal).clone()).await {
                        Some(gc) => gc,
                        None => {
                            let _ = tx.send(failed_result("REJECTED", "adapter rejected the goal"));
                            return;
                        }
                    };
                    let mut stream = goal_client.stream();
                    let mut final_result = failed_result("INTERNAL", "no result from adapter");
                    while let Some(event) = stream.next().await {
                        match event {
                            GoalEvent::Feedback(fb) => {
                                let mut out = DeviceTask_Feedback::default();
                                out.state = "running".into();
                                out.progress = fb.progress;
                                out.phase = fb.phase;
                                let _ = feedback_pub.publish(out);
                            }
                            GoalEvent::Status(_) => {}
                            GoalEvent::Result((_status, result)) => {
                                final_result = result;
                                break;
                            }
                        }
                    }
                    let _ = tx.send(final_result);
                });

                let executing = accepted.execute();
                let final_result = rx
                    .await
                    .unwrap_or_else(|_| failed_result("INTERNAL", "forwarding task dropped"));

                let succeeded = final_result.final_state == "succeeded";
                {
                    let mut o = orch.lock().unwrap();
                    let outcome = if succeeded {
                        TaskOutcome::succeeded()
                    } else {
                        TaskOutcome::failed(
                            final_result.error_code.clone(),
                            final_result.message.clone(),
                        )
                    };
                    o.complete(&goal.goal_id, outcome);
                }

                if succeeded {
                    executing.succeeded_with(final_result)
                } else {
                    executing.aborted_with(final_result)
                }
            }
        },
    )?;

    executor.spin(SpinOptions::default()).first_error()?;
    Ok(())
}
