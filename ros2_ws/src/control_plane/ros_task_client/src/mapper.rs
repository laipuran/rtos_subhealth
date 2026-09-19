use rclrs::GoalStatusCode;
use ros_env::{
    builtin_interfaces::msg::Time,
    task_interfaces::action::{ExecuteTask_Feedback, ExecuteTask_Goal, ExecuteTask_Result},
};
use serde::{Deserialize, Serialize};

use crate::{
    ExecuteCommand, FeedbackState, FinalState, PrimitiveCommand, PrimitiveDetails, RosTaskError,
    RosTimestamp, TaskFeedback, TaskResult,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoToTagPayload {
    target_tag: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoToTagDetails {
    current_tag: i32,
    next_tag: i32,
}

#[allow(clippy::field_reassign_with_default)]
pub(crate) fn to_ros_goal(command: &ExecuteCommand) -> Result<ExecuteTask_Goal, RosTaskError> {
    if command.task_id.is_empty() {
        return Err(RosTaskError::InvalidCommand {
            field: "task_id",
            message: "must not be empty".into(),
        });
    }
    if command.device_id.is_empty() {
        return Err(RosTaskError::InvalidCommand {
            field: "device_id",
            message: "must not be empty".into(),
        });
    }
    if command
        .deadline_unix_ms
        .is_some_and(|deadline| deadline <= 0)
    {
        return Err(RosTaskError::InvalidCommand {
            field: "deadline_unix_ms",
            message: "must be positive when provided".into(),
        });
    }

    let (primitive, payload_json) = match command.primitive {
        PrimitiveCommand::Hold => ("hold", "{}".into()),
        PrimitiveCommand::GoToTag { target_tag } => {
            let payload_json =
                serde_json::to_string(&GoToTagPayload { target_tag }).map_err(|error| {
                    RosTaskError::InvalidCommand {
                        field: "primitive",
                        message: format!("could not serialize go_to_tag payload: {error}"),
                    }
                })?;
            ("go_to_tag", payload_json)
        }
    };

    let mut goal = ExecuteTask_Goal::default();
    goal.task_id = command.task_id.clone();
    goal.device_id = command.device_id.clone();
    goal.primitive = primitive.into();
    goal.payload_json = payload_json;
    goal.deadline_unix_ms = command.deadline_unix_ms.unwrap_or(0);
    Ok(goal)
}

pub(crate) fn from_ros_feedback(
    expected_task_id: &str,
    primitive: &PrimitiveCommand,
    raw: ExecuteTask_Feedback,
) -> Result<TaskFeedback, RosTaskError> {
    validate_task_id(expected_task_id, &raw.task_id)?;

    let state = match raw.state.as_str() {
        "running" => FeedbackState::Running,
        "canceled" => FeedbackState::Canceled,
        state => {
            return Err(mapping_error(
                "state",
                format!("unknown feedback state `{state}`"),
            ));
        }
    };

    if !raw.progress.is_finite() || !(0.0..=1.0).contains(&raw.progress) {
        return Err(mapping_error(
            "progress",
            format!("must be finite and in [0.0, 1.0], got {}", raw.progress),
        ));
    }

    let details = match primitive {
        PrimitiveCommand::Hold => {
            let details: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&raw.details_json).map_err(|error| {
                    mapping_error("details_json", format!("invalid hold details: {error}"))
                })?;
            if !details.is_empty() {
                return Err(mapping_error(
                    "details_json",
                    "hold details must be an empty object",
                ));
            }
            PrimitiveDetails::Hold
        }
        PrimitiveCommand::GoToTag { .. } => {
            let details: GoToTagDetails =
                serde_json::from_str(&raw.details_json).map_err(|error| {
                    mapping_error(
                        "details_json",
                        format!("invalid go_to_tag details: {error}"),
                    )
                })?;
            PrimitiveDetails::GoToTag {
                current_tag: map_optional_tag("current_tag", details.current_tag)?,
                next_tag: map_optional_tag("next_tag", details.next_tag)?,
            }
        }
    };
    let timestamp = map_timestamp("timestamp", raw.timestamp)?;

    Ok(TaskFeedback {
        task_id: raw.task_id,
        state,
        progress: raw.progress,
        phase: raw.phase,
        details,
        timestamp,
    })
}

pub(crate) fn from_ros_result(
    expected_task_id: &str,
    status: GoalStatusCode,
    raw: ExecuteTask_Result,
) -> Result<TaskResult, RosTaskError> {
    validate_task_id(expected_task_id, &raw.task_id)?;

    let final_state = match raw.final_state.as_str() {
        "succeeded" => FinalState::Succeeded,
        "failed" => FinalState::Failed,
        "canceled" => FinalState::Canceled,
        state => {
            return Err(mapping_error(
                "final_state",
                format!("unknown final state `{state}`"),
            ));
        }
    };
    let status_final_state = match status {
        GoalStatusCode::Succeeded => FinalState::Succeeded,
        GoalStatusCode::Cancelled => FinalState::Canceled,
        GoalStatusCode::Aborted => FinalState::Failed,
        status => {
            return Err(mapping_error(
                "status",
                format!("ROS goal status {status:?} is not terminal"),
            ));
        }
    };
    if final_state != status_final_state {
        return Err(mapping_error(
            "status",
            format!(
                "ROS goal status {status:?} does not match final state `{}`",
                raw.final_state
            ),
        ));
    }

    let finished_time = map_timestamp("finished_time", raw.finished_time)?;
    let error_code = if raw.error_code.is_empty() {
        None
    } else {
        Some(raw.error_code)
    };

    Ok(TaskResult {
        task_id: raw.task_id,
        final_state,
        error_code,
        message: raw.message,
        finished_time,
    })
}

fn validate_task_id(expected_task_id: &str, actual_task_id: &str) -> Result<(), RosTaskError> {
    if expected_task_id.is_empty() || actual_task_id.is_empty() {
        return Err(mapping_error("task_id", "must not be empty"));
    }
    if actual_task_id != expected_task_id {
        return Err(mapping_error(
            "task_id",
            format!("expected `{expected_task_id}`, got `{actual_task_id}`"),
        ));
    }
    Ok(())
}

fn map_optional_tag(field: &'static str, tag: i32) -> Result<Option<i32>, RosTaskError> {
    match tag {
        -1 => Ok(None),
        0.. => Ok(Some(tag)),
        _ => Err(mapping_error(
            "details_json",
            format!("{field} must be -1 or non-negative, got {tag}"),
        )),
    }
}

fn map_timestamp(field: &'static str, timestamp: Time) -> Result<RosTimestamp, RosTaskError> {
    if timestamp.nanosec >= 1_000_000_000 {
        return Err(mapping_error(
            field,
            format!(
                "nanosec must be below one billion, got {}",
                timestamp.nanosec
            ),
        ));
    }
    Ok(RosTimestamp {
        sec: timestamp.sec,
        nanosec: timestamp.nanosec,
    })
}

fn mapping_error(field: &'static str, message: impl Into<String>) -> RosTaskError {
    RosTaskError::Mapping {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use rclrs::GoalStatusCode;
    use ros_env::{
        builtin_interfaces::msg::Time,
        task_interfaces::action::{ExecuteTask_Feedback, ExecuteTask_Result},
    };

    use crate::{
        ExecuteCommand, FeedbackState, FinalState, PrimitiveCommand, PrimitiveDetails,
        RosTaskError, RosTimestamp, TaskFeedback, TaskResult,
    };

    use super::{from_ros_feedback, from_ros_result, to_ros_goal};

    fn hold_command() -> ExecuteCommand {
        ExecuteCommand {
            task_id: "task-1".into(),
            device_id: "mock_exec".into(),
            primitive: PrimitiveCommand::Hold,
            deadline_unix_ms: None,
        }
    }

    fn go_to_tag_feedback() -> ExecuteTask_Feedback {
        ExecuteTask_Feedback {
            task_id: "task-2".into(),
            state: "running".into(),
            progress: 0.5,
            phase: "moving_to_tag".into(),
            details_json: r#"{"current_tag":1,"next_tag":-1}"#.into(),
            timestamp: Time {
                sec: 10,
                nanosec: 20,
            },
        }
    }

    fn result(final_state: &str, error_code: &str) -> ExecuteTask_Result {
        ExecuteTask_Result {
            task_id: "task-2".into(),
            final_state: final_state.into(),
            error_code: error_code.into(),
            message: "exec finished".into(),
            finished_time: Time {
                sec: 30,
                nanosec: 40,
            },
        }
    }

    #[test]
    fn hold_maps_to_empty_payload_and_zero_deadline() {
        let raw = to_ros_goal(&hold_command()).unwrap();

        assert_eq!(raw.task_id, "task-1");
        assert_eq!(raw.device_id, "mock_exec");
        assert_eq!(raw.primitive, "hold");
        assert_eq!(raw.payload_json, "{}");
        assert_eq!(raw.deadline_unix_ms, 0);
    }

    #[test]
    fn go_to_tag_maps_to_compact_payload() {
        let raw = to_ros_goal(&ExecuteCommand {
            task_id: "task-2".into(),
            device_id: "mock_exec".into(),
            primitive: PrimitiveCommand::GoToTag { target_tag: 42 },
            deadline_unix_ms: Some(1_789_700_000_000),
        })
        .unwrap();

        assert_eq!(raw.task_id, "task-2");
        assert_eq!(raw.device_id, "mock_exec");
        assert_eq!(raw.primitive, "go_to_tag");
        assert_eq!(raw.payload_json, r#"{"target_tag":42}"#);
        assert_eq!(raw.deadline_unix_ms, 1_789_700_000_000);
    }

    #[test]
    fn goal_rejects_empty_task_id() {
        let mut command = hold_command();
        command.task_id.clear();

        assert!(matches!(
            to_ros_goal(&command),
            Err(RosTaskError::InvalidCommand {
                field: "task_id",
                ..
            })
        ));
    }

    #[test]
    fn goal_rejects_empty_device_id() {
        let mut command = hold_command();
        command.device_id.clear();

        assert!(matches!(
            to_ros_goal(&command),
            Err(RosTaskError::InvalidCommand {
                field: "device_id",
                ..
            })
        ));
    }

    #[test]
    fn goal_rejects_non_positive_explicit_deadline() {
        for deadline_unix_ms in [0, -1] {
            let mut command = hold_command();
            command.deadline_unix_ms = Some(deadline_unix_ms);

            assert!(matches!(
                to_ros_goal(&command),
                Err(RosTaskError::InvalidCommand {
                    field: "deadline_unix_ms",
                    ..
                })
            ));
        }
    }

    #[test]
    fn go_to_tag_feedback_maps_literal_fields_and_tag_sentinel() {
        let mapped = from_ros_feedback(
            "task-2",
            &PrimitiveCommand::GoToTag { target_tag: 42 },
            go_to_tag_feedback(),
        )
        .unwrap();

        assert_eq!(
            mapped,
            TaskFeedback {
                task_id: "task-2".into(),
                state: FeedbackState::Running,
                progress: 0.5,
                phase: "moving_to_tag".into(),
                details: PrimitiveDetails::GoToTag {
                    current_tag: Some(1),
                    next_tag: None,
                },
                timestamp: RosTimestamp {
                    sec: 10,
                    nanosec: 20,
                },
            }
        );
    }

    #[test]
    fn hold_feedback_maps_empty_details_and_canceled_state() {
        let raw = ExecuteTask_Feedback {
            task_id: "task-1".into(),
            state: "canceled".into(),
            progress: 1.0,
            phase: "stopped".into(),
            details_json: "{}".into(),
            timestamp: Time {
                sec: -1,
                nanosec: 0,
            },
        };

        let mapped = from_ros_feedback("task-1", &PrimitiveCommand::Hold, raw).unwrap();

        assert_eq!(mapped.state, FeedbackState::Canceled);
        assert_eq!(mapped.details, PrimitiveDetails::Hold);
        assert_eq!(
            mapped.timestamp,
            RosTimestamp {
                sec: -1,
                nanosec: 0,
            }
        );
    }

    #[test]
    fn feedback_rejects_mismatched_or_empty_task_id() {
        let primitive = PrimitiveCommand::GoToTag { target_tag: 42 };

        assert!(matches!(
            from_ros_feedback("other-task", &primitive, go_to_tag_feedback()),
            Err(RosTaskError::Mapping {
                field: "task_id",
                ..
            })
        ));

        let mut raw = go_to_tag_feedback();
        raw.task_id.clear();
        assert!(matches!(
            from_ros_feedback("", &primitive, raw),
            Err(RosTaskError::Mapping {
                field: "task_id",
                ..
            })
        ));
    }

    #[test]
    fn feedback_rejects_go_to_tag_details_without_current_tag() {
        let mut raw = go_to_tag_feedback();
        raw.details_json = r#"{"next_tag":2}"#.into();

        assert!(matches!(
            from_ros_feedback("task-2", &PrimitiveCommand::GoToTag { target_tag: 42 }, raw),
            Err(RosTaskError::Mapping {
                field: "details_json",
                ..
            })
        ));
    }

    #[test]
    fn feedback_rejects_go_to_tag_details_without_next_tag() {
        let mut raw = go_to_tag_feedback();
        raw.details_json = r#"{"current_tag":1}"#.into();

        assert!(matches!(
            from_ros_feedback("task-2", &PrimitiveCommand::GoToTag { target_tag: 42 }, raw),
            Err(RosTaskError::Mapping {
                field: "details_json",
                ..
            })
        ));
    }

    #[test]
    fn feedback_rejects_wrong_typed_go_to_tag_fields() {
        for details_json in [
            r#"{"current_tag":"1","next_tag":2}"#,
            r#"{"current_tag":true,"next_tag":2}"#,
            r#"{"current_tag":1.5,"next_tag":2}"#,
            r#"{"current_tag":2147483648,"next_tag":2}"#,
            r#"{"current_tag":1,"next_tag":"2"}"#,
            r#"{"current_tag":1,"next_tag":false}"#,
            r#"{"current_tag":1,"next_tag":2.5}"#,
            r#"{"current_tag":1,"next_tag":2147483648}"#,
        ] {
            let mut raw = go_to_tag_feedback();
            raw.details_json = details_json.into();

            assert!(matches!(
                from_ros_feedback("task-2", &PrimitiveCommand::GoToTag { target_tag: 42 }, raw),
                Err(RosTaskError::Mapping {
                    field: "details_json",
                    ..
                })
            ));
        }
    }

    #[test]
    fn feedback_rejects_malformed_extra_or_wrong_primitive_details() {
        let invalid_cases = [
            (PrimitiveCommand::GoToTag { target_tag: 42 }, "not-json"),
            (
                PrimitiveCommand::GoToTag { target_tag: 42 },
                r#"{"current_tag":1,"next_tag":2,"extra":3}"#,
            ),
            (PrimitiveCommand::Hold, r#"{"current_tag":1,"next_tag":2}"#),
            (PrimitiveCommand::Hold, "[]"),
        ];

        for (primitive, details_json) in invalid_cases {
            let mut raw = go_to_tag_feedback();
            raw.details_json = details_json.into();

            assert!(matches!(
                from_ros_feedback("task-2", &primitive, raw),
                Err(RosTaskError::Mapping {
                    field: "details_json",
                    ..
                })
            ));
        }
    }

    #[test]
    fn feedback_rejects_unknown_state() {
        let mut raw = go_to_tag_feedback();
        raw.state = "paused".into();

        assert!(matches!(
            from_ros_feedback("task-2", &PrimitiveCommand::GoToTag { target_tag: 42 }, raw),
            Err(RosTaskError::Mapping { field: "state", .. })
        ));
    }

    #[test]
    fn feedback_rejects_non_finite_or_out_of_range_progress() {
        for progress in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.01, 1.01] {
            let mut raw = go_to_tag_feedback();
            raw.progress = progress;

            assert!(matches!(
                from_ros_feedback("task-2", &PrimitiveCommand::GoToTag { target_tag: 42 }, raw),
                Err(RosTaskError::Mapping {
                    field: "progress",
                    ..
                })
            ));
        }
    }

    #[test]
    fn feedback_rejects_tags_below_sentinel() {
        for details_json in [
            r#"{"current_tag":-2,"next_tag":1}"#,
            r#"{"current_tag":1,"next_tag":-2}"#,
        ] {
            let mut raw = go_to_tag_feedback();
            raw.details_json = details_json.into();

            assert!(matches!(
                from_ros_feedback("task-2", &PrimitiveCommand::GoToTag { target_tag: 42 }, raw),
                Err(RosTaskError::Mapping {
                    field: "details_json",
                    ..
                })
            ));
        }
    }

    #[test]
    fn feedback_rejects_timestamp_nanoseconds_at_or_above_one_billion() {
        for nanosec in [1_000_000_000, 1_000_000_001] {
            let mut raw = go_to_tag_feedback();
            raw.timestamp.nanosec = nanosec;

            assert!(matches!(
                from_ros_feedback("task-2", &PrimitiveCommand::GoToTag { target_tag: 42 }, raw),
                Err(RosTaskError::Mapping {
                    field: "timestamp",
                    ..
                })
            ));
        }
    }

    #[test]
    fn result_maps_succeeded_with_empty_error_code() {
        let mapped =
            from_ros_result("task-2", GoalStatusCode::Succeeded, result("succeeded", "")).unwrap();

        assert_eq!(
            mapped,
            TaskResult {
                task_id: "task-2".into(),
                final_state: FinalState::Succeeded,
                error_code: None,
                message: "exec finished".into(),
                finished_time: RosTimestamp {
                    sec: 30,
                    nanosec: 40,
                },
            }
        );
    }

    #[test]
    fn result_maps_cancelled_status_to_canceled_final_state() {
        let mapped =
            from_ros_result("task-2", GoalStatusCode::Cancelled, result("canceled", "")).unwrap();

        assert_eq!(mapped.final_state, FinalState::Canceled);
        assert_eq!(mapped.error_code, None);
    }

    #[test]
    fn failed_result_preserves_sdk_error() {
        let mapped = from_ros_result(
            "task-2",
            GoalStatusCode::Aborted,
            result("failed", "SDK_ERROR"),
        )
        .unwrap();

        assert_eq!(mapped.final_state, FinalState::Failed);
        assert_eq!(mapped.error_code.as_deref(), Some("SDK_ERROR"));
        assert_eq!(mapped.message, "exec finished");
    }

    #[test]
    fn result_rejects_every_terminal_status_and_final_state_mismatch() {
        let invalid_pairs = [
            (GoalStatusCode::Succeeded, "failed"),
            (GoalStatusCode::Succeeded, "canceled"),
            (GoalStatusCode::Cancelled, "succeeded"),
            (GoalStatusCode::Cancelled, "failed"),
            (GoalStatusCode::Aborted, "succeeded"),
            (GoalStatusCode::Aborted, "canceled"),
        ];

        for (status, final_state) in invalid_pairs {
            assert!(matches!(
                from_ros_result("task-2", status, result(final_state, "")),
                Err(RosTaskError::Mapping {
                    field: "status",
                    ..
                })
            ));
        }
    }

    #[test]
    fn result_rejects_nonterminal_ros_statuses() {
        for status in [
            GoalStatusCode::Unknown,
            GoalStatusCode::Accepted,
            GoalStatusCode::Executing,
            GoalStatusCode::Cancelling,
        ] {
            assert!(matches!(
                from_ros_result("task-2", status, result("succeeded", "")),
                Err(RosTaskError::Mapping {
                    field: "status",
                    ..
                })
            ));
        }
    }

    #[test]
    fn result_rejects_unknown_final_state() {
        assert!(matches!(
            from_ros_result("task-2", GoalStatusCode::Succeeded, result("done", "")),
            Err(RosTaskError::Mapping {
                field: "final_state",
                ..
            })
        ));
    }

    #[test]
    fn result_rejects_mismatched_or_empty_task_id() {
        assert!(matches!(
            from_ros_result(
                "other-task",
                GoalStatusCode::Succeeded,
                result("succeeded", "")
            ),
            Err(RosTaskError::Mapping {
                field: "task_id",
                ..
            })
        ));

        let mut raw = result("succeeded", "");
        raw.task_id.clear();
        assert!(matches!(
            from_ros_result("", GoalStatusCode::Succeeded, raw),
            Err(RosTaskError::Mapping {
                field: "task_id",
                ..
            })
        ));
    }

    #[test]
    fn result_rejects_timestamp_nanoseconds_at_or_above_one_billion() {
        for nanosec in [1_000_000_000, 1_000_000_001] {
            let mut raw = result("succeeded", "");
            raw.finished_time.nanosec = nanosec;

            assert!(matches!(
                from_ros_result("task-2", GoalStatusCode::Succeeded, raw),
                Err(RosTaskError::Mapping {
                    field: "finished_time",
                    ..
                })
            ));
        }
    }
}
