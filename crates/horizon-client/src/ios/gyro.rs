//! Gyro aiming on iOS (Options → Controls → Touch Controls → Gyroscope): CoreMotion's rotation
//! rate (`horizon-ios-sys`) through the Android port's conversion (`android/device_math.rs`), so
//! sensitivity and the turned screen behave as they do there.

/// The view's turn this frame, in degrees (yaw, pitch); zero, and the sensor off, while
/// `enabled` is false.
pub(crate) fn delta(enabled: bool, sensitivity: f32, seconds: f32) -> [f32; 2] {
    match horizon_ios_sys::motion::rate(enabled) {
        Some(rate) if enabled => crate::gyro_math::gyro_delta(
            rate,
            seconds,
            horizon_ios_sys::motion::interface_rotation(),
            sensitivity,
        ),
        _ => [0.0; 2],
    }
}
