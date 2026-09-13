use perception_sim::{Camera, Pose2d, SimulatedAprilTagDetector, Tag};

fn detector(noise: f64) -> SimulatedAprilTagDetector {
    SimulatedAprilTagDetector::new(Camera::default(), noise)
}

#[test]
fn detects_tag_straight_ahead() {
    let mut d = detector(0.0);
    let tags = [Tag {
        id: 42,
        x: 1.0,
        y: 0.0,
        z: 0.3,
    }];
    let detections = d.detect(Pose2d::new(0.0, 0.0, 0.0), &tags);
    assert_eq!(detections.len(), 1);
    let det = &detections[0];
    assert_eq!(det.id, 42);
    assert!((det.distance_mm - 1000.0).abs() < 1.0);
    assert!(det.center_offset_x.abs() < 1e-6);
    assert_eq!(det.hamming, 0);
}

#[test]
fn ignores_tag_behind() {
    let mut d = detector(0.0);
    let tags = [Tag {
        id: 1,
        x: -1.0,
        y: 0.0,
        z: 0.3,
    }];
    assert!(d.detect(Pose2d::new(0.0, 0.0, 0.0), &tags).is_empty());
}

#[test]
fn culls_tag_outside_frustum() {
    let mut d = detector(0.0);
    // 80 degrees to the left is outside the 90 degree H-FOV half-angle (45 deg).
    let tags = [Tag {
        id: 1,
        x: 1.0,
        y: 5.0,
        z: 0.3,
    }];
    assert!(d.detect(Pose2d::new(0.0, 0.0, 0.0), &tags).is_empty());
}

#[test]
fn culls_tag_out_of_range() {
    let mut d = detector(0.0);
    let tags = [Tag {
        id: 1,
        x: 10.0,
        y: 0.0,
        z: 0.3,
    }];
    assert!(d.detect(Pose2d::new(0.0, 0.0, 0.0), &tags).is_empty());
}

#[test]
fn respects_robot_orientation() {
    let mut d = detector(0.0);
    let tags = [Tag {
        id: 7,
        x: 0.0,
        y: 1.0,
        z: 0.3,
    }];
    // Robot faces +y (yaw = 90deg): the tag is straight ahead.
    let detections = d.detect(Pose2d::new(0.0, 0.0, std::f64::consts::FRAC_PI_2), &tags);
    assert_eq!(detections.len(), 1);
    assert!(detections[0].center_offset_x.abs() < 1e-6);
}

#[test]
fn noise_is_deterministic_and_bounded() {
    let tags = [Tag {
        id: 1,
        x: 1.0,
        y: 0.0,
        z: 0.3,
    }];
    let mut a = detector(0.05);
    let mut b = detector(0.05);
    let da = a.detect(Pose2d::new(0.0, 0.0, 0.0), &tags);
    let db = b.detect(Pose2d::new(0.0, 0.0, 0.0), &tags);
    assert_eq!(da, db);
    assert!(da[0].center_offset_x.abs() <= 1.0);
}
