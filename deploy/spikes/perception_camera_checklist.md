# Hardware validation: camera AprilTag perception

**Purpose.** Confirm `perception_camera` detects real `tag36h11` markers and
publishes RFC-001 `AprilTagDetections`.

## Prerequisites

- A camera publishing `sensor_msgs/Image` (mono8 / rgb8 / bgr8) on
  `/camera/device` (or set `CAMERA_TOPIC`).
- The `ros-dev:jazzy` image with the built workspace.

## Steps

```bash
# 1. Start the detector
CAMERA_TOPIC=/camera/device HFOV_DEG=90 TAG_SIZE_M=0.16 \
  ros2 run perception_camera perception_camera

# 2. In another shell, watch detections
ros2 topic echo /perception/apriltag_detections
```

Hold a printed `tag36h11` marker (id known) in front of the camera.

## Expected

- Detections with the correct `id` and `hamming` near 0.
- `distance` in mm roughly matches the real distance (scale depends on the true
  `HFOV_DEG` / `TAG_SIZE_M`; calibrate these for accuracy).
- Empty `detections` array when no tag is visible.

## Calibration notes

- For accurate `distance`/angles, provide real intrinsics. The node currently
  approximates `fx` from `HFOV_DEG`; replace with a `CameraInfo` subscription
  when calibration data is available.
