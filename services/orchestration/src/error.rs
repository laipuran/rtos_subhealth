#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrchestrationError {
    #[error("no device satisfies the task requirements")]
    NoDevice,
    #[error("device is busy")]
    Busy,
    #[error("task already exists")]
    Duplicate,
    #[error("task does not exist")]
    UnknownTask,
    #[error("execution port error: {0}")]
    Execution(String),
}
