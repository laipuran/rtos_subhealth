use device_sdk::{Command, Velocity};
use safety::{SafetyLimits, SafetySupervisor, SafetyViolation};

fn supervisor() -> SafetySupervisor {
    SafetySupervisor::new(
        SafetyLimits {
            max_speed_mps: 1.0,
            max_yaw_rate_rps: 2.0,
        },
        5.0,
    )
}

#[test]
fn clamps_linear_magnitude() {
    let s = supervisor();
    let clamped = s.clamp_velocity(Velocity::new(3.0, 4.0, 0.0));
    let magnitude = (clamped.vx.powi(2) + clamped.vy.powi(2)).sqrt();
    assert!((magnitude - 1.0).abs() < 1e-9);
    // Direction preserved.
    assert!((clamped.vx / clamped.vy - 0.75).abs() < 1e-9);
}

#[test]
fn clamps_yaw_rate() {
    let s = supervisor();
    let clamped = s.clamp_velocity(Velocity::new(0.5, 0.0, 10.0));
    assert_eq!(clamped.vyaw, 2.0);
}

#[test]
fn estop_blocks_commands() {
    let mut s = supervisor();
    s.heartbeat(0.0);
    s.engage_estop();
    assert_eq!(s.check(0.1), Err(SafetyViolation::EStop));
    assert!(s.sanitize(Command::Stop, 0.1).is_err());
    s.release_estop();
    assert!(s.check(0.1).is_ok());
}

#[test]
fn watchdog_flags_stale_state() {
    let mut s = supervisor();
    s.heartbeat(100.0);
    assert!(!s.is_stale(103.0));
    assert!(s.is_stale(106.0));
    assert_eq!(s.check(106.0), Err(SafetyViolation::Stale(6.0)));
    s.heartbeat(106.0);
    assert!(s.check(106.0).is_ok());
}

#[test]
fn sanitize_clamps_velocity_commands() {
    let mut s = supervisor();
    s.heartbeat(0.0);
    let cmd = s
        .sanitize(
            Command::SetVelocity {
                velocity: Velocity::new(5.0, 0.0, 9.0),
                duration_s: 1.0,
            },
            0.1,
        )
        .unwrap();
    match cmd {
        Command::SetVelocity { velocity, .. } => {
            assert!(velocity.vx <= 1.0 + 1e-9);
            assert_eq!(velocity.vyaw, 2.0);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn no_watchdog_timeout_disables_staleness() {
    let mut s = SafetySupervisor::new(SafetyLimits::default(), 0.0);
    s.heartbeat(0.0);
    assert!(!s.is_stale(1_000_000.0));
}
