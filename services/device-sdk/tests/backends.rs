use device_sdk::{
    BackendStatus, Command, DeviceBackend, DiffDriveSim, MockDevice, Pose2d, Velocity,
};
use orchestrator_core::Primitive;

fn run_until_terminal(backend: &mut dyn DeviceBackend, dt: f64, max_steps: usize) -> BackendStatus {
    for _ in 0..max_steps {
        let status = backend.step(dt);
        if status.is_terminal() {
            return status;
        }
    }
    panic!("backend did not reach a terminal state");
}

#[test]
fn diff_drive_reaches_target_pose() {
    let mut sim = DiffDriveSim::new("base");
    sim.command(Command::MoveToPose {
        target: Pose2d::new(2.0, 0.0, 0.0),
        position_tolerance_m: 0.1,
        yaw_tolerance_rad: 0.2,
    })
    .unwrap();

    let status = run_until_terminal(&mut sim, 0.05, 2000);
    assert_eq!(status, BackendStatus::Succeeded);
    let pose = sim.pose().unwrap();
    assert!((pose.x - 2.0).abs() < 0.2, "x = {}", pose.x);
    assert!(pose.y.abs() < 0.2, "y = {}", pose.y);
}

#[test]
fn diff_drive_rejects_unsupported_primitive() {
    let mut sim = DiffDriveSim::new("base");
    let err = sim.command(Command::ExecutePrimitive {
        name: "wave".into(),
        params: serde_json::json!({}),
    });
    assert!(err.is_err());
}

#[test]
fn stop_zeroes_velocity() {
    let mut sim = DiffDriveSim::new("base");
    sim.command(Command::SetVelocity {
        velocity: Velocity::new(0.3, 0.0, 0.0),
        duration_s: 0.0,
    })
    .unwrap();
    sim.step(0.1);
    sim.command(Command::Stop).unwrap();
    assert_eq!(sim.velocity().unwrap(), Velocity::ZERO);
}

#[test]
fn mock_succeeds_after_configured_steps() {
    let mut mock = MockDevice::new("tonypi", &[Primitive::ExecutePrimitive], 3);
    mock.command(Command::ExecutePrimitive {
        name: "wave".into(),
        params: serde_json::json!({}),
    })
    .unwrap();
    assert_eq!(
        run_until_terminal(&mut mock, 0.1, 10),
        BackendStatus::Succeeded
    );
}

#[test]
fn mock_failure_is_reported() {
    let mut mock = MockDevice::new("tonypi", &[Primitive::Hold], 2).failing();
    mock.command(Command::Hold { duration_s: 0.0 }).unwrap();
    assert!(matches!(
        run_until_terminal(&mut mock, 0.1, 10),
        BackendStatus::Failed { .. }
    ));
}

#[test]
fn mock_rejects_unsupported_primitive() {
    let mut mock = MockDevice::new("tonypi", &[Primitive::ExecutePrimitive], 1);
    assert!(mock
        .command(Command::MoveToPose {
            target: Pose2d::new(1.0, 0.0, 0.0),
            position_tolerance_m: 0.1,
            yaw_tolerance_rad: 0.1,
        })
        .is_err());
}
