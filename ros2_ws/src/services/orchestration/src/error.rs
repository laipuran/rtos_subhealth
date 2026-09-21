#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrchestrationError {
    #[error("device is busy")]
    Busy,
    #[error("task already exists")]
    Duplicate,
    #[error("task does not exist")]
    UnknownTask,
    #[error("task is already terminal")]
    TerminalTask,
    #[error("task target must not be empty")]
    InvalidTarget,
    #[error("task repository error: {0}")]
    Repository(String),
    #[error("execution port error: {0}")]
    Execution(String),
}
