//! Geometry-based AprilTag detection.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose2d {
    pub x: f64,
    pub y: f64,
    pub yaw: f64,
}

impl Pose2d {
    pub fn new(x: f64, y: f64, yaw: f64) -> Self {
        Self { x, y, yaw }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub hfov_rad: f64,
    pub vfov_rad: f64,
    pub min_range_m: f64,
    pub max_range_m: f64,
    pub height_m: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            hfov_rad: 90.0_f64.to_radians(),
            vfov_rad: 70.0_f64.to_radians(),
            min_range_m: 0.3,
            max_range_m: 5.0,
            height_m: 0.3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tag {
    pub id: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub id: i32,
    pub distance_mm: f32,
    pub center_offset_x: f32,
    pub center_offset_y: f32,
    pub roll: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub hamming: i32,
}

pub struct SimulatedAprilTagDetector {
    camera: Camera,
    noise: f64,
    rng: u64,
}

impl SimulatedAprilTagDetector {
    pub fn new(camera: Camera, noise: f64) -> Self {
        Self {
            camera,
            noise,
            rng: 0x9E3779B97F4A7C15,
        }
    }

    fn uniform(&mut self) -> f64 {
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.rng >> 11) as f64) / ((1u64 << 53) as f64) * 2.0 - 1.0
    }

    pub fn detect(&mut self, robot: Pose2d, tags: &[Tag]) -> Vec<Detection> {
        let mut out = Vec::new();
        let (s, c) = robot.yaw.sin_cos();
        for tag in tags {
            let dx = tag.x - robot.x;
            let dy = tag.y - robot.y;
            // Rotate world delta into the robot frame (x forward, y left).
            let forward = c * dx + s * dy;
            let left = -s * dx + c * dy;
            let up = tag.z - self.camera.height_m;

            if forward <= 0.0 {
                continue;
            }
            let range = (forward * forward + left * left + up * up).sqrt();
            if range < self.camera.min_range_m || range > self.camera.max_range_m {
                continue;
            }
            let angle_h = (left / forward).atan();
            let angle_v = (up / forward).atan();
            if angle_h.abs() > self.camera.hfov_rad / 2.0
                || angle_v.abs() > self.camera.vfov_rad / 2.0
            {
                continue;
            }

            let mut offset_x = angle_h / (self.camera.hfov_rad / 2.0);
            let mut offset_y = angle_v / (self.camera.vfov_rad / 2.0);
            let mut distance = range;
            let mut yaw_deg = angle_h.to_degrees();
            if self.noise > 0.0 {
                offset_x = (offset_x + self.uniform() * self.noise).clamp(-1.0, 1.0);
                offset_y = (offset_y + self.uniform() * self.noise).clamp(-1.0, 1.0);
                distance *= 1.0 + self.uniform() * self.noise * 0.02;
                yaw_deg += self.uniform() * self.noise * 25.0;
            }

            out.push(Detection {
                id: tag.id,
                distance_mm: (distance * 1000.0) as f32,
                center_offset_x: offset_x as f32,
                center_offset_y: offset_y as f32,
                roll: 0.0,
                yaw: yaw_deg as f32,
                pitch: angle_v.to_degrees() as f32,
                hamming: 0,
            });
        }
        out
    }
}
