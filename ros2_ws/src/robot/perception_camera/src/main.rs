//! Real-camera AprilTag detection node (tag36h11).
//!
//! Subscribes to a `sensor_msgs/Image` (mono8 / rgb8 / bgr8), runs the official
//! AprilTag detector, and publishes `AprilTagDetections` (RFC-001). Intrinsics
//! are approximated from a configurable horizontal FOV; a full calibration can
//! replace this later.
//!
//! Environment:
//!   CAMERA_TOPIC   image topic (default /camera/device)
//!   HFOV_DEG       horizontal field of view (default 90)
//!   TAG_SIZE_M     physical tag edge length (default 0.16)

use std::cell::RefCell;

use anyhow::Result;
use apriltag::{Detection, Detector, Family, Image as TagImage};
use rclrs::*;

use perception_interfaces::msg::{AprilTagDetection, AprilTagDetections};

thread_local! {
    /// `Detector` is not `Send`, so each subscription worker thread owns one.
    static DETECTOR: RefCell<Option<Detector>> = const { RefCell::new(None) };
}

fn detect(image: &TagImage) -> Vec<Detection> {
    DETECTOR.with(|cell| {
        let mut guard = cell.borrow_mut();
        if guard.is_none() {
            *guard = Detector::builder()
                .add_family_bits(Family::tag_36h11(), 2)
                .build()
                .ok();
        }
        match guard.as_mut() {
            Some(detector) => detector.detect(image),
            None => Vec::new(),
        }
    })
}

/// Convert a ROS image to a single-channel gray buffer (row-major).
fn to_gray(msg: &sensor_msgs::msg::Image) -> Option<(usize, usize, Vec<u8>)> {
    let w = msg.width as usize;
    let h = msg.height as usize;
    let step = msg.step as usize;
    let data = &msg.data;
    if w == 0 || h == 0 || data.len() < step * h {
        return None;
    }
    let mut gray = vec![0u8; w * h];
    match msg.encoding.as_str() {
        "mono8" => {
            for y in 0..h {
                gray[y * w..y * w + w].copy_from_slice(&data[y * step..y * step + w]);
            }
        }
        "rgb8" | "bgr8" => {
            let rgb = msg.encoding == "rgb8";
            for y in 0..h {
                let row = &data[y * step..y * step + w * 3];
                for x in 0..w {
                    let (r, g, b) = if rgb {
                        (row[x * 3], row[x * 3 + 1], row[x * 3 + 2])
                    } else {
                        (row[x * 3 + 2], row[x * 3 + 1], row[x * 3])
                    };
                    gray[y * w + x] =
                        (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8;
                }
            }
        }
        _ => return None,
    }
    Some((w, h, gray))
}

fn main() -> Result<()> {
    let context = Context::default_from_env()?;
    let mut executor = context.create_basic_executor();
    let node = executor.create_node("perception_camera")?;

    let camera_topic = std::env::var("CAMERA_TOPIC").unwrap_or_else(|_| "/camera/device".into());
    let hfov_deg: f64 = std::env::var("HFOV_DEG")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(90.0);
    let tag_size: f64 = std::env::var("TAG_SIZE_M")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.16);

    let publisher = node.create_publisher::<AprilTagDetections>("/perception/apriltag_detections")?;

    let _subscription = node.create_subscription::<sensor_msgs::msg::Image, _>(
        camera_topic.as_str(),
        move |msg: sensor_msgs::msg::Image| {
            let Some((w, h, gray)) = to_gray(&msg) else {
                return;
            };
            let mut image = match TagImage::zeros_with_stride(w, h, w) {
                Ok(image) => image,
                Err(_) => return,
            };
            image.as_slice_mut().copy_from_slice(&gray);

            let cx = w as f64 / 2.0;
            let cy = h as f64 / 2.0;
            let fx = cx / (hfov_deg.to_radians() / 2.0).tan();
            let vfov_deg = 2.0 * (cy / fx).atan().to_degrees();

            let detections = detect(&image);
            let mut out = AprilTagDetections::default();
            out.frame_id = msg.header.frame_id.clone();
            out.detections = detections
                .iter()
                .map(|d| {
                    let center = d.center();
                    let corners = d.corners();
                    let width_px = ((corners[1][0] - corners[0][0]).powi(2)
                        + (corners[1][1] - corners[0][1]).powi(2))
                    .sqrt();
                    let distance_m = if width_px > 1.0 {
                        tag_size * fx / width_px
                    } else {
                        0.0
                    };
                    let offset_x = (center[0] - cx) / cx;
                    let offset_y = (center[1] - cy) / cy;

                    let mut det = AprilTagDetection::default();
                    det.id = d.id() as i32;
                    det.distance = (distance_m * 1000.0) as f32;
                    det.center_offset_x = offset_x as f32;
                    det.center_offset_y = offset_y as f32;
                    det.roll = 0.0;
                    det.yaw = (offset_x * hfov_deg / 2.0) as f32;
                    det.pitch = (-offset_y * vfov_deg / 2.0) as f32;
                    det.hamming = d.hamming() as i32;
                    det
                })
                .collect();
            let _ = publisher.publish(&out);
        },
    )?;

    executor.spin(SpinOptions::default()).first_error()?;
    Ok(())
}
