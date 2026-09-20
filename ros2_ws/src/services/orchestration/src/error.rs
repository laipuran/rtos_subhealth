#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrchestrationError {
    #[error("device is busy")]
    Busy,
    #[error("task already exists")]
    Duplicate,
    #[error("task does not exist")]
    UnknownTask,
    #[error("execution port error: {0}")]
    Execution(String),
}
