use orchestrator_core::{DeviceDescriptor, DeviceKind, DeviceLimits, Primitive};

pub fn descriptor(
    id: &str,
    kind: DeviceKind,
    primitives: &[Primitive],
    capabilities: &[&str],
) -> DeviceDescriptor {
    DeviceDescriptor {
        device_id: id.into(),
        name: id.into(),
        kind,
        capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
        primitives: primitives.iter().map(|p| p.as_str().to_string()).collect(),
        limits: DeviceLimits::default(),
        frames: vec!["map".into(), "base_link".into()],
        sensors: vec![],
    }
}

pub fn wheeled(id: &str) -> DeviceDescriptor {
    descriptor(
        id,
        DeviceKind::WheeledDiff,
        &[
            Primitive::MoveToPose,
            Primitive::SetVelocity,
            Primitive::Hold,
            Primitive::Stop,
        ],
        &["supports_velocity", "supports_pose"],
    )
}

/// A servo-only humanoid like TonyPi: no pose, no velocity, only named actions.
pub fn servo_humanoid(id: &str) -> DeviceDescriptor {
    descriptor(
        id,
        DeviceKind::Legged,
        &[
            Primitive::ExecutePrimitive,
            Primitive::Hold,
            Primitive::Stop,
        ],
        &["supports_action_groups"],
    )
}
