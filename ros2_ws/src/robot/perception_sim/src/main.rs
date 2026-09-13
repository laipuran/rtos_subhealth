//! Simulated AprilTag perception node.
//!
//! Loads tag positions from the world-model map, reads the latest robot pose
//! from `/device_states`, and publishes `AprilTagDetections` at 10 Hz using the
//! geometry-based detector from `perception-sim`.
//!
//! Environment:
//!   MAP_PATH  path to the world-model JSON (default: config/maps/default.json)

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use rclrs::*;

use device_interfaces::msg::DeviceState;
use perception_interfaces::msg::{AprilTagDetection, AprilTagDetections};
use perception_sim::{Camera, Detection, Pose2d, SimulatedAprilTagDetector, Tag};

fn quat_to_yaw(q: &geometry_msgs::msg::Quaternion) -> f64 {
    let (x, y, z, w) = (q.x, q.y, q.z, q.w);
    (2.0 * (w * z + x * y)).atan2(1.0 - 2.0 * (y * y + z * z))
}

fn load_tags(path: &str) -> Vec<Tag> {
    let Ok(json) = std::fs::read_to_string(path) else {
        eprintln!("[perception_sim] map not found: {path}");
        return Vec::new();
    };
    match world_model::WorldModel::from_json_str(&json) {
        Ok(model) => model
            .nodes
            .values()
            .map(|n| Tag {
                id: n.id,
                x: n.x,
                y: n.y,
                z: 0.3,
            })
            .collect(),
        Err(err) => {
            eprintln!("[perception_sim] invalid map: {err}");
            Vec::new()
        }
    }
}

fn to_msg(d: &Detection) -> AprilTagDetection {
    let mut m = AprilTagDetection::default();
    m.id = d.id;
    m.distance = d.distance_mm;
    m.center_offset_x = d.center_offset_x;
    m.center_offset_y = d.center_offset_y;
    m.roll = d.roll;
    m.yaw = d.yaw;
    m.pitch = d.pitch;
    m.hamming = d.hamming;
    m
}

fn main() -> Result<()> {
    let context = Context::default_from_env()?;
    let mut executor = context.create_basic_executor();
    let node = executor.create_node("perception_sim")?;

    let map_path =
        std::env::var("MAP_PATH").unwrap_or_else(|_| "config/maps/default.json".into());
    let tags = load_tags(&map_path);

    let pose: Arc<Mutex<Pose2d>> = Arc::new(Mutex::new(Pose2d::new(0.0, 0.0, 0.0)));
    let pose_sub = Arc::clone(&pose);
    let _pose_subscription = node.create_subscription::<DeviceState, _>(
        "/device_states",
        move |msg: DeviceState| {
            let mut p = pose_sub.lock().unwrap();
            *p = Pose2d::new(
                msg.pose.pose.position.x,
                msg.pose.pose.position.y,
                quat_to_yaw(&msg.pose.pose.orientation),
            );
        },
    )?;

    let publisher = node.create_publisher::<AprilTagDetections>("/perception/apriltag_detections")?;
    let mut detector = SimulatedAprilTagDetector::new(Camera::default(), 0.02);
    let _timer = node.create_timer_repeating(Duration::from_millis(100), move || {
        let robot = *pose.lock().unwrap();
        let detections = detector.detect(robot, &tags);
        let mut msg = AprilTagDetections::default();
        msg.frame_id = "camera_link".into();
        msg.detections = detections.iter().map(to_msg).collect();
        let _ = publisher.publish(&msg);
    })?;

    executor.spin(SpinOptions::default()).first_error()?;
    Ok(())
}
