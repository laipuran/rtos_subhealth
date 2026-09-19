use tokio::sync::{mpsc, oneshot};

use crate::RosTaskError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteCommand {
    pub task_id: String,
    pub device_id: String,
    pub primitive: PrimitiveCommand,
    pub deadline_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveCommand {
    Hold,
    GoToTag { target_tag: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackState {
    Running,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveDetails {
    Hold,
    GoToTag {
        current_tag: Option<i32>,
        next_tag: Option<i32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RosTimestamp {
    pub sec: i32,
    pub nanosec: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskFeedback {
    pub task_id: String,
    pub state: FeedbackState,
    pub progress: f32,
    pub phase: String,
    pub details: PrimitiveDetails,
    pub timestamp: RosTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalState {
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    pub task_id: String,
    pub final_state: FinalState,
    pub error_code: Option<String>,
    pub message: String,
    pub finished_time: RosTimestamp,
}

pub struct TaskSession {
    pub task_id: String,
    pub feedback: mpsc::Receiver<Result<TaskFeedback, RosTaskError>>,
    pub result: oneshot::Receiver<Result<TaskResult, RosTaskError>>,
    pub cancellation: CancellationHandle,
}

#[derive(Clone)]
pub struct CancellationHandle {
    request_tx: mpsc::Sender<CancelRequest>,
    task_id: String,
}

impl CancellationHandle {
    pub(crate) fn new(request_tx: mpsc::Sender<CancelRequest>, task_id: String) -> Self {
        Self {
            request_tx,
            task_id,
        }
    }

    pub async fn cancel(&self) -> Result<(), RosTaskError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.request_tx
            .send(CancelRequest { response_tx })
            .await
            .map_err(|_| RosTaskError::CancelRejected {
                task_id: self.task_id.clone(),
                reason: "relay is no longer accepting cancellation requests".into(),
            })?;
        response_rx
            .await
            .map_err(|_| RosTaskError::CancelRejected {
                task_id: self.task_id.clone(),
                reason: "relay stopped before the cancellation response was delivered".into(),
            })?
    }
}

pub(crate) struct CancelRequest {
    pub(crate) response_tx: oneshot::Sender<Result<(), RosTaskError>>,
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::CancellationHandle;
    use crate::RosTaskError;

    #[tokio::test]
    async fn closed_relay_returns_typed_cancellation_unavailable_error() {
        let (request_tx, request_rx) = mpsc::channel(1);
        drop(request_rx);
        let cancellation = CancellationHandle::new(request_tx, "task-1".into());

        assert!(matches!(
            cancellation.cancel().await,
            Err(RosTaskError::CancelRejected { task_id, reason })
                if task_id == "task-1" && reason.contains("no longer accepting")
        ));
    }
}
