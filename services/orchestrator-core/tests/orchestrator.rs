mod common;

use common::{servo_humanoid, wheeled};
use orchestrator_core::registry::DeviceRegistry;
use orchestrator_core::{
    Orchestrator, Primitive, RejectReason, Task, TaskOutcome, TaskState, TaskTarget,
};

fn registry() -> DeviceRegistry {
    let mut reg = DeviceRegistry::new();
    reg.register(wheeled("base")).unwrap();
    reg.register(servo_humanoid("tonypi")).unwrap();
    reg
}

fn task(goal_id: &str, device_id: &str, primitive: Primitive, target: Option<TaskTarget>) -> Task {
    Task {
        goal_id: goal_id.into(),
        device_id: device_id.into(),
        primitive,
        target,
        params_json: None,
        deadline_ms: 0,
    }
}

#[test]
fn unknown_device_is_rejected() {
    let mut orch = Orchestrator::new(registry());
    let err = orch
        .submit(task("g1", "ghost", Primitive::Hold, None))
        .unwrap_err();
    assert_eq!(err, RejectReason::UnknownDevice("ghost".into()));
}

#[test]
fn unsupported_primitive_is_rejected_for_servo_humanoid() {
    let mut orch = Orchestrator::new(registry());
    let err = orch
        .submit(task(
            "g1",
            "tonypi",
            Primitive::MoveToPose,
            Some(TaskTarget::pose(0.05, 0.1)),
        ))
        .unwrap_err();
    assert_eq!(
        err,
        RejectReason::UnsupportedPrimitive {
            device_id: "tonypi".into(),
            primitive: Primitive::MoveToPose,
        }
    );
}

#[test]
fn servo_humanoid_accepts_named_action() {
    let mut orch = Orchestrator::new(registry());
    let plan = orch
        .submit(task(
            "g1",
            "tonypi",
            Primitive::ExecutePrimitive,
            Some(TaskTarget::action("wave")),
        ))
        .unwrap();
    assert_eq!(plan.device_id, "tonypi");
    assert_eq!(orch.active_count(), 1);
}

#[test]
fn move_to_pose_requires_pose_target() {
    let mut orch = Orchestrator::new(registry());
    let err = orch
        .submit(task("g1", "base", Primitive::MoveToPose, None))
        .unwrap_err();
    assert!(matches!(err, RejectReason::InvalidTarget { .. }));

    let err = orch
        .submit(task(
            "g1",
            "base",
            Primitive::MoveToPose,
            Some(TaskTarget::tag(42)),
        ))
        .unwrap_err();
    assert!(matches!(err, RejectReason::InvalidTarget { .. }));
}

#[test]
fn busy_device_is_rejected() {
    let mut orch = Orchestrator::new(registry());
    orch.submit(task(
        "g1",
        "base",
        Primitive::MoveToPose,
        Some(TaskTarget::pose(0.05, 0.1)),
    ))
    .unwrap();
    let err = orch
        .submit(task("g2", "base", Primitive::Hold, None))
        .unwrap_err();
    assert_eq!(err, RejectReason::DeviceBusy("base".into()));
}

#[test]
fn duplicate_goal_is_rejected() {
    let mut orch = Orchestrator::new(registry());
    orch.submit(task("g1", "base", Primitive::Hold, None))
        .unwrap();
    let err = orch
        .submit(task("g1", "tonypi", Primitive::Hold, None))
        .unwrap_err();
    assert_eq!(err, RejectReason::DuplicateGoal("g1".into()));
}

#[test]
fn lifecycle_promotes_and_releases_device() {
    let mut orch = Orchestrator::new(registry());
    orch.submit(task("g1", "base", Primitive::Hold, None))
        .unwrap();
    assert_eq!(orch.state("g1").unwrap().state, TaskState::Dispatched);

    orch.on_feedback("g1", 0.4, "holding");
    let active = orch.state("g1").unwrap();
    assert_eq!(active.state, TaskState::Running);
    assert_eq!(active.progress, 0.4);
    assert_eq!(active.phase, "holding");

    let done = orch.complete("g1", TaskOutcome::succeeded()).unwrap();
    assert_eq!(done.state, TaskState::Succeeded);
    assert!(orch.state("g1").is_none());
    assert!(orch.active_for_device("base").is_none());

    // Device is free again.
    orch.submit(task("g2", "base", Primitive::Hold, None))
        .unwrap();
}

#[test]
fn cancel_releases_device() {
    let mut orch = Orchestrator::new(registry());
    orch.submit(task("g1", "base", Primitive::Hold, None))
        .unwrap();
    assert!(orch.cancel("g1"));
    assert!(!orch.cancel("g1"));
    assert!(orch.active_for_device("base").is_none());
}
