//! Single-task orchestrator: validate a task against the device registry, emit
//! a dispatch plan, and track its lifecycle. No ROS dependency.

use std::collections::HashMap;

use crate::device::Primitive;
use crate::registry::DeviceRegistry;
use crate::task::{TargetKind, Task, TaskOutcome, TaskState, TaskTarget};

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchPlan {
    pub goal_id: String,
    pub device_id: String,
    pub primitive: Primitive,
    pub target: Option<TaskTarget>,
    pub params_json: Option<String>,
    pub deadline_ms: i64,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RejectReason {
    #[error("unknown device: {0}")]
    UnknownDevice(String),
    #[error("device {device_id} does not support primitive {}", primitive.as_str())]
    UnsupportedPrimitive {
        device_id: String,
        primitive: Primitive,
    },
    #[error("device {0} is busy")]
    DeviceBusy(String),
    #[error("duplicate goal_id: {0}")]
    DuplicateGoal(String),
    #[error("invalid target for primitive {}: {reason}", primitive.as_str())]
    InvalidTarget {
        primitive: Primitive,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveTask {
    pub plan: DispatchPlan,
    pub state: TaskState,
    pub progress: f32,
    pub phase: String,
    pub error_code: String,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct Orchestrator {
    registry: DeviceRegistry,
    active: HashMap<String, ActiveTask>,
    device_to_goal: HashMap<String, String>,
}

impl Orchestrator {
    pub fn new(registry: DeviceRegistry) -> Self {
        Self {
            registry,
            active: HashMap::new(),
            device_to_goal: HashMap::new(),
        }
    }

    pub fn registry(&self) -> &DeviceRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut DeviceRegistry {
        &mut self.registry
    }

    /// Validate and reserve a device for `task`, returning the plan to execute.
    pub fn submit(&mut self, task: Task) -> Result<DispatchPlan, RejectReason> {
        if self.active.contains_key(&task.goal_id) {
            return Err(RejectReason::DuplicateGoal(task.goal_id));
        }
        self.validate(&task)?;
        if self.device_to_goal.contains_key(&task.device_id) {
            return Err(RejectReason::DeviceBusy(task.device_id));
        }

        let plan = DispatchPlan {
            goal_id: task.goal_id.clone(),
            device_id: task.device_id.clone(),
            primitive: task.primitive,
            target: task.target.clone(),
            params_json: task.params_json.clone(),
            deadline_ms: task.deadline_ms,
        };
        self.device_to_goal
            .insert(task.device_id.clone(), task.goal_id.clone());
        self.active.insert(
            task.goal_id.clone(),
            ActiveTask {
                plan: plan.clone(),
                state: TaskState::Dispatched,
                progress: 0.0,
                phase: String::new(),
                error_code: String::new(),
                message: String::new(),
            },
        );
        Ok(plan)
    }

    fn validate(&self, task: &Task) -> Result<(), RejectReason> {
        let device = self
            .registry
            .get(&task.device_id)
            .ok_or_else(|| RejectReason::UnknownDevice(task.device_id.clone()))?;
        if !device.supports_primitive(task.primitive) {
            return Err(RejectReason::UnsupportedPrimitive {
                device_id: task.device_id.clone(),
                primitive: task.primitive,
            });
        }

        let invalid = |reason: &str| RejectReason::InvalidTarget {
            primitive: task.primitive,
            reason: reason.to_string(),
        };

        match task.primitive {
            Primitive::MoveToPose => match &task.target {
                Some(t) if t.kind == TargetKind::Pose => {}
                Some(_) => return Err(invalid("move_to_pose requires a pose target")),
                None => return Err(invalid("move_to_pose requires a target")),
            },
            Primitive::ExecutePrimitive => {
                let has_action = matches!(&task.target, Some(t) if t.kind == TargetKind::Action);
                if !has_action && task.params_json.is_none() {
                    return Err(invalid(
                        "execute_primitive requires an action target or params_json",
                    ));
                }
            }
            Primitive::SetVelocity | Primitive::Hold | Primitive::Stop => {}
        }
        Ok(())
    }

    /// Record progress from an adapter. Promotes a dispatched task to running.
    pub fn on_feedback(
        &mut self,
        goal_id: &str,
        progress: f32,
        phase: &str,
    ) -> Option<&ActiveTask> {
        let active = self.active.get_mut(goal_id)?;
        if active.state == TaskState::Dispatched {
            active.state = TaskState::Running;
        }
        active.progress = progress.clamp(0.0, 1.0);
        active.phase = phase.to_string();
        Some(active)
    }

    /// Finalize a task and release its device.
    pub fn complete(&mut self, goal_id: &str, outcome: TaskOutcome) -> Option<ActiveTask> {
        let mut active = self.active.remove(goal_id)?;
        self.device_to_goal.remove(&active.plan.device_id);
        active.state = outcome.state;
        active.error_code = outcome.error_code;
        active.message = outcome.message;
        Some(active)
    }

    pub fn cancel(&mut self, goal_id: &str) -> bool {
        let Some(active) = self.active.remove(goal_id) else {
            return false;
        };
        self.device_to_goal.remove(&active.plan.device_id);
        true
    }

    pub fn state(&self, goal_id: &str) -> Option<&ActiveTask> {
        self.active.get(goal_id)
    }

    pub fn active_for_device(&self, device_id: &str) -> Option<&ActiveTask> {
        self.device_to_goal
            .get(device_id)
            .and_then(|goal_id| self.active.get(goal_id))
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}
