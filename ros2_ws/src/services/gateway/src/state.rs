use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use futures_util::StreamExt;
use orchestration::{ActiveTask, OrchestrationError, Orchestrator};
use platform::{
    ExecutionFeedback, ExecutionResult, ExecutionSession, SystemEvent, TaskId, TaskState,
};
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::dto::{CreateTask, TaskView};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

pub struct AppStateInner {
    tasks: RwLock<HashMap<TaskId, TaskView>>,
    task_sequence: AtomicU64,
    event_sequence: AtomicU64,
    events: broadcast::Sender<(u64, SystemEvent)>,
    orchestrator: Mutex<Orchestrator>,
}

impl AppState {
    pub fn new(orchestrator: Orchestrator) -> Self {
        let (events, _) = broadcast::channel(128);
        Self {
            inner: Arc::new(AppStateInner {
                tasks: RwLock::new(HashMap::new()),
                task_sequence: AtomicU64::new(0),
                event_sequence: AtomicU64::new(0),
                events,
                orchestrator: Mutex::new(orchestrator),
            }),
        }
    }

    pub async fn create_task(&self, input: CreateTask) -> Result<TaskView, OrchestrationError> {
        let id = self.next_task_id();
        let task = input.into_task(id.clone());
        let session = self
            .inner
            .orchestrator
            .lock()
            .await
            .submit(task.clone())
            .await?;
        let view = TaskView {
            task,
            state: TaskState::Accepted,
            progress: 0.0,
            phase: "accepted".into(),
        };

        self.inner.tasks.write().await.insert(id, view.clone());
        self.emit_task_state(&view.task.id, view.state.clone());
        self.consume_session(view.task.id.clone(), session);
        Ok(view)
    }

    pub async fn list_tasks(&self) -> Vec<TaskView> {
        self.inner.tasks.read().await.values().cloned().collect()
    }

    pub async fn task(&self, id: &TaskId) -> Option<TaskView> {
        self.inner.tasks.read().await.get(id).cloned()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<(u64, SystemEvent)> {
        self.inner.events.subscribe()
    }

    fn next_task_id(&self) -> TaskId {
        TaskId(format!(
            "task-{}",
            self.inner.task_sequence.fetch_add(1, Ordering::Relaxed) + 1
        ))
    }

    fn consume_session(&self, task_id: TaskId, session: ExecutionSession) {
        let state = self.clone();
        tokio::spawn(async move {
            state.apply_session(task_id, session).await;
        });
    }

    async fn apply_session(&self, task_id: TaskId, session: ExecutionSession) {
        let ExecutionSession {
            mut feedback,
            result,
        } = session;

        while let Some(feedback) = feedback.next().await {
            match feedback {
                Ok(feedback) => self.apply_feedback(feedback).await,
                Err(error) => {
                    self.apply_execution_error(&task_id, error).await;
                    return;
                }
            }
        }

        match result.await {
            Ok(result) => self.apply_result(result).await,
            Err(error) => self.apply_execution_error(&task_id, error).await,
        }
    }

    async fn apply_execution_error(&self, task_id: &TaskId, error: platform::ExecutionError) {
        tracing::warn!(%error, task_id = %task_id.0, "execution session failed");
        self.apply_result(ExecutionResult {
            task_id: task_id.clone(),
            state: "failed".into(),
        })
        .await;
    }

    async fn apply_feedback(&self, feedback: ExecutionFeedback) {
        let active = match self.inner.orchestrator.lock().await.feedback(feedback) {
            Ok(active) => active.clone(),
            Err(error) => {
                tracing::warn!(%error, "unable to apply execution feedback");
                return;
            }
        };
        self.update_projection(active).await;
    }

    async fn apply_result(&self, result: ExecutionResult) {
        let active = match self.inner.orchestrator.lock().await.complete(result) {
            Ok(active) => active,
            Err(error) => {
                tracing::warn!(%error, "unable to apply execution result");
                return;
            }
        };
        self.update_projection(active).await;
    }

    async fn update_projection(&self, active: ActiveTask) {
        let state_changed = {
            let mut tasks = self.inner.tasks.write().await;
            let Some(view) = tasks.get_mut(&active.task.id) else {
                tracing::warn!(task_id = %active.task.id.0, "task projection is missing");
                return;
            };
            let state_changed = view.state != active.state;
            view.state = active.state.clone();
            view.progress = active.progress;
            view.phase = active.phase;
            state_changed
        };

        if state_changed {
            self.emit_task_state(&active.task.id, active.state);
        }
    }

    fn emit_task_state(&self, task_id: &TaskId, state: TaskState) {
        let sequence = self.inner.event_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = self.inner.events.send((
            sequence,
            SystemEvent::TaskStateChanged {
                task_id: task_id.clone(),
                state,
            },
        ));
    }
}
