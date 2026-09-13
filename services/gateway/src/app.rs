//! Application state and router assembly.

use std::sync::Arc;

use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

use crate::api::ws::EventHub;
use crate::bridge::RosBridge;
use crate::config::Config;
use crate::store::diagnosis_store::DiagnosisStore;
use crate::store::task_store::TaskStore;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub tasks: Arc<TaskStore>,
    pub diagnoses: Arc<DiagnosisStore>,
    pub hub: EventHub,
    pub bridge: Arc<dyn RosBridge>,
}

impl AppState {
    pub fn new(config: Config, bridge: Arc<dyn RosBridge>) -> anyhow::Result<Self> {
        let tasks = Arc::new(TaskStore::open(&config.db_dir)?);
        let diagnoses = Arc::new(DiagnosisStore::open(&config.db_dir)?);
        Ok(Self {
            config: Arc::new(config),
            tasks,
            diagnoses,
            hub: EventHub::new(256),
            bridge,
        })
    }
}

/// Build the full application router, optionally serving the built WebUI.
pub fn build_router(state: AppState) -> Router {
    let app = crate::api::http::router().with_state(state.clone());
    match &state.config.webui_dir {
        Some(dir) => {
            let index = dir.join("index.html");
            app.fallback_service(
                ServeDir::new(dir.clone()).not_found_service(ServeFile::new(index)),
            )
        }
        None => app,
    }
}
