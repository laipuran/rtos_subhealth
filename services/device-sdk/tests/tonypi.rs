use std::sync::{Arc, Mutex};

use device_sdk::tonypi::{ActionRunner, TonyPiBackend};
use device_sdk::{BackendStatus, Command, DeviceBackend, Velocity};
use orchestrator_core::Primitive;
use serde_json::json;

#[derive(Clone, Default)]
struct RecordingRunner {
    calls: Arc<Mutex<Vec<String>>>,
    fail: bool,
}

impl ActionRunner for RecordingRunner {
    fn run_action(&mut self, action: &str) -> Result<(), String> {
        self.calls.lock().unwrap().push(action.to_string());
        if self.fail {
            Err("robot offline".into())
        } else {
            Ok(())
        }
    }
}

#[test]
fn descriptor_advertises_only_action_primitives() {
    let backend = TonyPiBackend::new("tonypi", RecordingRunner::default());
    let d = backend.descriptor();
    assert_eq!(d.device_id, "tonypi");
    assert!(d.supports_primitive(Primitive::Stop));
    assert!(d.primitives.contains(&"execute_primitive".to_string()));
    assert!(!d.primitives.contains(&"move_to_pose".to_string()));
    assert_eq!(backend.pose(), None);
}

#[test]
fn execute_primitive_runs_action_group() {
    let runner = RecordingRunner::default();
    let calls = Arc::clone(&runner.calls);
    let mut backend = TonyPiBackend::new("tonypi", runner);
    backend
        .command(Command::ExecutePrimitive {
            name: "wave".into(),
            params: json!({"action": "wave"}),
        })
        .unwrap();
    assert_eq!(backend.step(0.1), BackendStatus::Succeeded);
    assert_eq!(&*calls.lock().unwrap(), &["wave"]);
}

#[test]
fn stop_runs_the_stop_action() {
    let runner = RecordingRunner::default();
    let calls = Arc::clone(&runner.calls);
    let mut backend = TonyPiBackend::new("tonypi", runner);
    backend.command(Command::Stop).unwrap();
    assert_eq!(backend.step(0.1), BackendStatus::Succeeded);
    assert_eq!(&*calls.lock().unwrap(), &["0"]);
}

#[test]
fn rejects_imu_only_or_mobile_primitives() {
    let mut backend = TonyPiBackend::new("tonypi", RecordingRunner::default());
    assert!(backend
        .command(Command::SetVelocity {
            velocity: Velocity::new(0.1, 0.0, 0.0),
            duration_s: 0.0,
        })
        .is_err());
}

#[test]
fn transport_failure_is_reported() {
    let mut backend = TonyPiBackend::new(
        "tonypi",
        RecordingRunner {
            fail: true,
            ..Default::default()
        },
    );
    backend
        .command(Command::ExecutePrimitive {
            name: "wave".into(),
            params: json!({}),
        })
        .unwrap();
    assert!(matches!(backend.step(0.1), BackendStatus::Failed { .. }));
}
