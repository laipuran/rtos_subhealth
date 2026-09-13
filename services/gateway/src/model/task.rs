//! Task domain model. Field names and JSON layout match the legacy Python
//! `TaskRecord`/`ExecTask.Goal` exactly, so existing `tasks.db` rows stay valid
//! and the WebUI contract is unchanged.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{ApiError, ErrorCode};

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

/// Device-agnostic task target (mirrors `device_interfaces/msg/TaskTarget`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskTarget {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub tag_id: i32,
    #[serde(default)]
    pub waypoint_id: String,
    #[serde(default)]
    pub action_id: String,
    #[serde(default)]
    pub position_tolerance_m: f32,
    #[serde(default)]
    pub yaw_tolerance_rad: f32,
}

impl TaskTarget {
    pub fn from_value(value: &Value) -> Result<Self, ApiError> {
        let target = value
            .as_object()
            .ok_or_else(|| invalid_goal("target must be an object"))?;
        Ok(Self {
            kind: optional_string(target.get("kind"), "kind")?.unwrap_or_default(),
            tag_id: match target.get("tag_id") {
                Some(tag_id) => tag_id_from_value(tag_id)?,
                None => 0,
            },
            waypoint_id: optional_string(target.get("waypoint_id"), "waypoint_id")?
                .unwrap_or_default(),
            action_id: optional_string(target.get("action_id"), "action_id")?.unwrap_or_default(),
            position_tolerance_m: optional_f32(target, "position_tolerance_m")?.unwrap_or(0.0),
            yaw_tolerance_rad: optional_f32(target, "yaw_tolerance_rad")?.unwrap_or(0.0),
        })
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
    /// Device-agnostic fields (empty for legacy tag-only goals).
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub primitive: String,
    #[serde(default)]
    pub target: Option<TaskTarget>,
    #[serde(default)]
    pub params_json: String,
}

impl Goal {
    pub fn from_canonical_value(value: &Value) -> Result<Self, ApiError> {
        let device_id = required_string(value, "device_id")?;
        let primitive = required_string(value, "primitive")?;
        let target = value
            .get("target")
            .map(TaskTarget::from_value)
            .transpose()?;
        if primitive == "move_to_pose"
            && !matches!(target.as_ref(), Some(target) if target.kind == "tag" && target.tag_id != 0)
        {
            return Err(ApiError::new(
                ErrorCode::InvalidGoal,
                "move_to_pose requires target.kind 'tag' with a non-zero tag_id",
            ));
        }
        let constraints = constraints_from_value(value)?;
        let deadline_ms = optional_i64(value, "deadline_ms")?.unwrap_or(0);
        let params_json =
            optional_string(value.get("params_json"), "params_json")?.unwrap_or_default();

        Ok(Self {
            type_: primitive.clone(),
            priority: 0,
            route_id: String::new(),
            target_tags: target
                .iter()
                .map(|item| item.tag_id)
                .filter(|id| *id != 0)
                .collect(),
            constraints,
            deadline_ms,
            device_id,
            primitive,
            target,
            params_json,
        })
    }

    pub fn from_legacy_value(value: &Value) -> Result<Self, ApiError> {
        let device_id = required_string(value, "target_device")?;
        let goal = value
            .get("goal")
            .ok_or_else(|| ApiError::new(ErrorCode::InvalidGoal, "missing goal"))?;
        let type_ = required_string(goal, "type")?;
        match type_.as_str() {
            "go_to_tag" => {
                let tags = goal
                    .get("target_tags")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        ApiError::new(ErrorCode::InvalidGoal, "go_to_tag requires one integer tag")
                    })?;
                if tags.len() != 1 {
                    return Err(ApiError::new(
                        ErrorCode::InvalidGoal,
                        "go_to_tag requires exactly one integer tag",
                    ));
                }
                let tag_id = tag_id_from_value(&tags[0])?;
                if tag_id == 0 {
                    return Err(invalid_goal("go_to_tag requires one non-zero integer tag"));
                }
                Ok(Self {
                    type_: "move_to_pose".to_string(),
                    priority: 0,
                    route_id: String::new(),
                    target_tags: vec![tag_id],
                    constraints: constraints_from_value(goal)?,
                    deadline_ms: optional_i64(goal, "deadline_ms")?.unwrap_or(0),
                    device_id,
                    primitive: "move_to_pose".to_string(),
                    target: Some(TaskTarget {
                        kind: "tag".to_string(),
                        tag_id,
                        ..Default::default()
                    }),
                    params_json: String::new(),
                })
            }
            "hold" => Ok(Self {
                type_: "hold".to_string(),
                priority: 0,
                route_id: String::new(),
                target_tags: Vec::new(),
                constraints: constraints_from_value(goal)?,
                deadline_ms: optional_i64(goal, "deadline_ms")?.unwrap_or(0),
                device_id,
                primitive: "hold".to_string(),
                target: None,
                params_json: String::new(),
            }),
            "patrol_route" => Err(ApiError::new(
                ErrorCode::InvalidGoal,
                "patrol_route is not supported",
            )),
            _ => Err(ApiError::new(
                ErrorCode::InvalidGoal,
                "unknown legacy goal type",
            )),
        }
    }

    pub fn new(type_: impl Into<String>) -> Self {
        Self {
            type_: type_.into(),
            priority: 0,
            route_id: String::new(),
            target_tags: Vec::new(),
            constraints: Constraints::default(),
            deadline_ms: 0,
            device_id: String::new(),
            primitive: String::new(),
            target: None,
            params_json: String::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<i32>) -> Self {
        self.target_tags = tags;
        self
    }

    pub fn with_device(
        mut self,
        device_id: impl Into<String>,
        primitive: impl Into<String>,
    ) -> Self {
        self.device_id = device_id.into();
        self.primitive = primitive.into();
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
            device_id: String::new(),
            primitive: String::new(),
            target: None,
            params_json: String::new(),
        })
    }

    /// Parse a device-agnostic task from a request body (the new shape). The
    /// device identifier may be omitted and defaults to `"mock"`.
    pub fn from_task_value(body: &Value) -> Option<Self> {
        let primitive = body.get("primitive")?.as_str()?.to_string();
        if primitive.is_empty() {
            return None;
        }
        let device_id = body
            .get("device_id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or("mock")
            .to_string();
        let target = body
            .get("target")
            .filter(|v| !v.is_null())
            .and_then(|target| TaskTarget::from_value(target).ok());
        let c = body.get("constraints").cloned().unwrap_or(Value::Null);
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
            type_: primitive.clone(),
            priority: body.get("priority").and_then(Value::as_i64).unwrap_or(0) as i32,
            route_id: body
                .get("route_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            target_tags: Vec::new(),
            constraints,
            deadline_ms: body.get("deadline_ms").and_then(Value::as_i64).unwrap_or(0),
            device_id,
            primitive,
            target,
            params_json: body
                .get("params_json")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        })
    }
}

fn required_string(value: &Value, field: &str) -> Result<String, ApiError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| ApiError::new(ErrorCode::InvalidGoal, format!("missing {field}")))
}

fn invalid_goal(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::InvalidGoal, message)
}

fn optional_string(value: Option<&Value>, field: &str) -> Result<Option<String>, ApiError> {
    value
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| invalid_goal(format!("{field} must be a string")))
        })
        .transpose()
}

fn optional_i64(value: &Value, field: &str) -> Result<Option<i64>, ApiError> {
    value
        .get(field)
        .map(|value| {
            value
                .as_i64()
                .ok_or_else(|| invalid_goal(format!("{field} must be an integer")))
        })
        .transpose()
}

fn optional_f32(
    value: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<f32>, ApiError> {
    value
        .get(field)
        .map(|value| {
            value
                .as_f64()
                .map(|value| value as f32)
                .ok_or_else(|| invalid_goal(format!("{field} must be a number")))
        })
        .transpose()
}

fn tag_id_from_value(value: &Value) -> Result<i32, ApiError> {
    value
        .as_i64()
        .ok_or_else(|| invalid_goal("tag ID must be an integer"))
        .and_then(|tag_id| {
            i32::try_from(tag_id).map_err(|_| invalid_goal("tag ID is outside the i32 range"))
        })
}

fn constraints_from_value(value: &Value) -> Result<Constraints, ApiError> {
    let Some(value) = value.get("constraints") else {
        return Ok(Constraints::default());
    };
    let constraints = value
        .as_object()
        .ok_or_else(|| invalid_goal("constraints must be an object"))?;
    let avoid_tags = match constraints.get("avoid_tags") {
        Some(value) => value
            .as_array()
            .ok_or_else(|| invalid_goal("constraints.avoid_tags must be an array"))?
            .iter()
            .map(tag_id_from_value)
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };
    Ok(Constraints {
        max_speed_mps: optional_f32(constraints, "max_speed_mps")?.unwrap_or(0.0),
        min_clearance_m: optional_f32(constraints, "min_clearance_m")?.unwrap_or(0.0),
        avoid_tags,
    })
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
        let target = serde_json::to_value(&self.goal.target).unwrap_or(Value::Null);
        json!({
            "goal_id": self.goal_id,
            "type": self.goal.type_,
            "priority": self.goal.priority,
            "route_id": self.goal.route_id,
            "target_tags": self.goal.target_tags,
            "device_id": self.goal.device_id,
            "primitive": self.goal.primitive,
            "target": target,
            "params_json": self.goal.params_json,
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
