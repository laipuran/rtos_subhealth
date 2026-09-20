use rclrs::GoalStatusCode;
use ros_env::task_interfaces::action::{
    ExecuteTask_Feedback, ExecuteTask_Goal, ExecuteTask_Result,
};
use serde::Serialize;

use crate::{ExecuteCommand, FinalState, PrimitiveCommand, RosTaskError, TaskFeedback, TaskResult};

#[derive(Serialize)]
struct GoToTagPayload<'a> {
    target_tags: &'a [i32],
}

pub fn to_ros_goal(command: &ExecuteCommand) -> Result<ExecuteTask_Goal, RosTaskError> {
    let deadline_unix_ms = command
        .deadline_ms
        .map(i64::try_from)
        .transpose()
        .map_err(|_| RosTaskError::InvalidCommand {
            field: "deadline_ms",
            message: "must fit the ROS int64 deadline".into(),
        })?
        .unwrap_or(0);
    let primitive = match command.primitive {
        PrimitiveCommand::GoToTag => "go_to_tag",
    };
    let payload_json = serde_json::to_string(&GoToTagPayload {
        target_tags: &command.target,
    })
    .map_err(|error| mapping_error("target", error.to_string()))?;
    Ok(ExecuteTask_Goal {
        task_id: command.task_id.clone(),
        device_id: command.device_id.clone(),
        primitive: primitive.into(),
        payload_json,
        deadline_unix_ms,
    })
}

pub fn from_ros_feedback(
    expected_task_id: &str,
    raw: ExecuteTask_Feedback,
) -> Result<TaskFeedback, RosTaskError> {
    validate_task_id(expected_task_id, &raw.task_id)?;
    if !raw.progress.is_finite() || !(0.0..=1.0).contains(&raw.progress) {
        return Err(mapping_error(
            "progress",
            "must be finite and in [0.0, 1.0]",
        ));
    }
    Ok(TaskFeedback {
        task_id: raw.task_id,
        progress: raw.progress,
        phase: raw.phase,
    })
}

pub fn from_ros_result(
    expected_task_id: &str,
    status: GoalStatusCode,
    raw: ExecuteTask_Result,
) -> Result<TaskResult, RosTaskError> {
    validate_task_id(expected_task_id, &raw.task_id)?;
    let final_state = match (status, raw.final_state.as_str()) {
        (GoalStatusCode::Succeeded, "succeeded") => FinalState::Succeeded,
        (GoalStatusCode::Aborted, "failed") => FinalState::Failed,
        (status, state) => {
            return Err(mapping_error(
                "final_state",
                format!("unsupported ROS status/final state pair: {status:?}/{state}"),
            ))
        }
    };
    Ok(TaskResult {
        task_id: raw.task_id,
        final_state,
    })
}

fn validate_task_id(expected: &str, actual: &str) -> Result<(), RosTaskError> {
    if actual != expected || actual.is_empty() {
        return Err(mapping_error(
            "task_id",
            format!("expected `{expected}`, got `{actual}`"),
        ));
    }
    Ok(())
}

fn mapping_error(field: &'static str, message: impl Into<String>) -> RosTaskError {
    RosTaskError::Mapping {
        field,
        message: message.into(),
    }
}
