use platform::SystemEvent;
use platform::TaskId;
use platform::TaskState;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::dto::CreateTask;
use crate::dto::TaskView;

#[derive(Clone)]
pub struct GatewayState {
    pub(crate) tasks: Arc<RwLock<HashMap<TaskId, TaskView>>>,
    pub(crate) sequence: Arc<AtomicU64>,
    pub(crate) events: broadcast::Sender<(u64, SystemEvent)>,
}

impl Default for GatewayState {
    fn default() -> Self {
        Self::new()
    }
}

impl GatewayState {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(128);
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            sequence: Arc::new(AtomicU64::new(0)),
            events,
        }
    }

    pub(crate) async fn create_task(&self, input: CreateTask) -> TaskView {
        let id = TaskId(format!(
            "task-{}",
            self.sequence.fetch_add(1, Ordering::Relaxed) + 1
        ));
        let task = input.into_task(id.clone());
        let view = TaskView {
            task,
            state: TaskState::Accepted,
            progress: 0.0,
            phase: "accepted".into(),
        };
        self.tasks.write().await.insert(id, view.clone());
        view
    }
}
