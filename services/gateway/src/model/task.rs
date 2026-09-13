//! Task domain model. Field names and JSON layout match the legacy Python
//! `TaskRecord`/`ExecTask.Goal` exactly, so existing `tasks.db` rows stay valid
//! and the WebUI contract is unchanged.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraints {
    pub max_speed_mps: f32,
    pub min_clearance_m: f32,
    pub avoid_tags: Vec<i32>,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            max_speed_mps: 0.0,
            min_clearance_m: 0.0,
            avoid_tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    #[serde(rename = "type")]
    pub type_: String,
    pub priority: i32,
    pub route_id: String,
    pub target_tags: Vec<i32>,
    #[serde(default)]
    pub constraints: Constraints,
    pub deadline_ms: i64,
}

impl Goal {
    pub fn new(type_: impl Into<String>) -> Self {
        Self {
            type_: type_.into(),
            priority: 0,
            route_id: String::new(),
            target_tags: Vec::new(),
            constraints: Constraints::default(),
            deadline_ms: 0,
        }
    }

    pub fn with_tags(mut self, tags: Vec<i32>) -> Self {
        self.target_tags = tags;
        self
    }

    /// Parse a goal from an HTTP request body, applying the same defaults as the
    /// legacy `_build_goal_from_body`.
    pub fn from_body_value(goal: &Value) -> Option<Self> {
        let type_ = goal.get("type")?.as_str()?.to_string();
        let target_tags = goal
            .get("target_tags")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_i64)
                    .map(|v| v as i32)
                    .collect()
            })
            .unwrap_or_default();
        let c = goal.get("constraints").cloned().unwrap_or(Value::Null);
        let constraints = Constraints {
            max_speed_mps: c
                .get("max_speed_mps")
                .and_then(Value::as_f64)
                .unwrap_or(0.0) as f32,
            min_clearance_m: c
                .get("min_clearance_m")
                .and_then(Value::as_f64)
                .unwrap_or(0.0) as f32,
            avoid_tags: c
                .get("avoid_tags")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_i64)
                        .map(|v| v as i32)
                        .collect()
                })
                .unwrap_or_default(),
        };
        Some(Self {
            type_,
            priority: goal.get("priority").and_then(Value::as_i64).unwrap_or(0) as i32,
            route_id: goal
                .get("route_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            target_tags,
            constraints,
            deadline_ms: goal.get("deadline_ms").and_then(Value::as_i64).unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub goal_id: String,
    pub goal: Goal,
    pub state: String,
    pub progress: f64,
    pub current_tag: i32,
    pub next_tag: i32,
    pub error_code: String,
    pub message: String,
    pub final_state: Option<String>,
    pub route: Vec<i32>,
    pub finished_stages: i64,
    pub created_at: f64,
    pub updated_at: f64,
}

impl TaskRecord {
    pub fn new(goal_id: impl Into<String>, goal: Goal) -> Self {
        let now = crate::store::now();
        Self {
            goal_id: goal_id.into(),
            goal,
            state: "accepted".into(),
            progress: 0.0,
            current_tag: -1,
            next_tag: -1,
            error_code: String::new(),
            message: String::new(),
            final_state: None,
            route: Vec::new(),
            finished_stages: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// The external wire representation returned by the HTTP API.
    pub fn to_wire_dict(&self) -> Value {
        json!({
            "goal_id": self.goal_id,
            "type": self.goal.type_,
            "priority": self.goal.priority,
            "route_id": self.goal.route_id,
            "target_tags": self.goal.target_tags,
            "state": self.state,
            "progress": self.progress,
            "current_tag": self.current_tag,
            "next_tag": self.next_tag,
            "error_code": self.error_code,
            "message": self.message,
            "final_state": self.final_state,
            "route": self.route,
            "finished_stages": self.finished_stages,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        })
    }
}

/// Partial update applied to a task record.
#[derive(Debug, Default, Clone)]
pub struct TaskUpdate {
    pub state: Option<String>,
    pub progress: Option<f64>,
    pub current_tag: Option<i32>,
    pub next_tag: Option<i32>,
    pub error_code: Option<String>,
    pub message: Option<String>,
    pub final_state: Option<String>,
    pub route: Option<Vec<i32>>,
    pub finished_stages: Option<i64>,
}

impl TaskUpdate {
    pub fn is_empty(&self) -> bool {
        self.state.is_none()
            && self.progress.is_none()
            && self.current_tag.is_none()
            && self.next_tag.is_none()
            && self.error_code.is_none()
            && self.message.is_none()
            && self.final_state.is_none()
            && self.route.is_none()
            && self.finished_stages.is_none()
    }
}
