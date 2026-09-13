use std::sync::Mutex;

use crate::bridge::{BridgeCommand, RosBridge};
use crate::model::task::Goal;

/// Test/in-process bridge that records the commands it is asked to emit.
#[derive(Default)]
pub struct MockBridge {
    commands: Mutex<Vec<BridgeCommand>>,
}

impl MockBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commands(&self) -> Vec<BridgeCommand> {
        self.commands.lock().unwrap().clone()
    }

    pub fn take(&self) -> Vec<BridgeCommand> {
        std::mem::take(&mut *self.commands.lock().unwrap())
    }
}

impl RosBridge for MockBridge {
    fn send_goal(&self, goal_id: &str, goal: Goal) {
        self.commands.lock().unwrap().push(BridgeCommand::SendGoal {
            goal_id: goal_id.to_string(),
            goal: Box::new(goal),
        });
    }

    fn cancel(&self, goal_id: &str) {
        self.commands.lock().unwrap().push(BridgeCommand::Cancel {
            goal_id: goal_id.to_string(),
        });
    }

    fn trigger_diagnosis(&self, diagnosis_id: &str) {
        self.commands
            .lock()
            .unwrap()
            .push(BridgeCommand::TriggerDiagnosis {
                diagnosis_id: diagnosis_id.to_string(),
            });
    }
}
