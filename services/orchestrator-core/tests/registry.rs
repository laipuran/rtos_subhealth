mod common;

use common::{servo_humanoid, wheeled};
use orchestrator_core::registry::{DeviceRegistry, RegistryError};
use orchestrator_core::Primitive;

#[test]
fn duplicate_registration_is_rejected() {
    let mut reg = DeviceRegistry::new();
    reg.register(wheeled("base")).unwrap();
    assert_eq!(
        reg.register(wheeled("base")),
        Err(RegistryError::Duplicate("base".into()))
    );
    assert_eq!(reg.len(), 1);
}

#[test]
fn unregister_missing_is_error() {
    let mut reg = DeviceRegistry::new();
    assert_eq!(
        reg.unregister("ghost"),
        Err(RegistryError::NotFound("ghost".into()))
    );
}

#[test]
fn find_filters_by_primitive_and_capability() {
    let mut reg = DeviceRegistry::new();
    reg.register(wheeled("base")).unwrap();
    reg.register(servo_humanoid("tonypi")).unwrap();

    let movers = reg.find(&[Primitive::MoveToPose], None);
    assert_eq!(movers.len(), 1);
    assert_eq!(movers[0].device_id, "base");

    let actors = reg.find(
        &[Primitive::ExecutePrimitive],
        Some("supports_action_groups"),
    );
    assert_eq!(actors.len(), 1);
    assert_eq!(actors[0].device_id, "tonypi");

    let none = reg.find(&[Primitive::SetVelocity], Some("supports_action_groups"));
    assert!(none.is_empty());
}
