use thiserror::Error;

impl From<RosTaskError> for platform::ExecutionError {
    fn from(error: RosTaskError) -> Self {
        Self::Failed(error.to_string())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RosTaskError {
    #[error("invalid configuration for {field}: {message}")]
    InvalidConfig {
        field: &'static str,
        message: String,
    },
    #[error("invalid command for {field}: {message}")]
    InvalidCommand {
        field: &'static str,
        message: String,
    },
    #[error("unknown device: {device_id}")]
    UnknownDevice { device_id: String },
    #[error("action server unavailable for device {device_id} at {action_name}")]
    ActionServerUnavailable {
        device_id: String,
        action_name: String,
    },
    #[error("goal rejected for task {task_id}")]
    GoalRejected { task_id: String },
    #[error("mapping error for {field}: {message}")]
    Mapping {
        field: &'static str,
        message: String,
    },
    #[error("feedback overflow for task {task_id}: channel capacity is {capacity}")]
    FeedbackOverflow { task_id: String, capacity: usize },
    /// rclrs 0.7 exposes its errors without a source shape compatible with
    /// this cloneable public error enum, so the Display text is retained at
    /// the boundary rather than wrapping a non-cloneable source.
    #[error("ROS error: {0}")]
    Ros(String),
    #[error("{channel} channel closed")]
    ChannelClosed { channel: &'static str },
    #[error("ROS task client is shutting down")]
    Shutdown,
    #[error("ROS executor panicked")]
    ExecutorPanicked,
}
