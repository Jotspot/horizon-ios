//! The iOS side of the platform calls the shared touch glue makes (`game/android_input.rs`), with
//! the same names as `android/platform.rs`. The system keyboard is not driven from here: on iOS it
//! follows the focused text field through winit's text input, as on desktop (`game/ime.rs`).
use horizon_client::osk::Output;
use horizon_client::{android_gamepad::InputMode, android_runtime};

// The input in use (touch, keyboard and mouse, or a controller) is tracked by the same shared
// state as on Android, so the touch controls show and hide the same way.
pub(crate) fn touch_input(touch: bool) {
    android_runtime::note_input(if touch {
        InputMode::Touch
    } else {
        InputMode::KeyboardMouse
    });
}
pub(crate) fn touch_visible() -> bool {
    android_runtime::active_input() == InputMode::Touch
}
pub(crate) fn keyboard_mouse_input() -> bool {
    android_runtime::active_input() == InputMode::KeyboardMouse
}

/// Whether the system keyboard is editing a field.
pub fn active() -> bool {
    false
}
pub fn show(_text: &str) {}
pub fn show_multiline(text: &str) {
    show(text);
}
pub fn hide() {}
/// The system keyboard's edits since the last call.
pub fn poll() -> Vec<Output> {
    Vec::new()
}
/// Field hit tests use the same GUI coordinates as the touch menus.
pub fn field_at(p: [f32; 2]) -> bool {
    horizon_client::widgets::last_targets().iter().any(|t| {
        let [x, y, w, h] = t.rect;
        t.kind == horizon_client::widgets::Kind::Field
            && (x..x + w).contains(&p[0])
            && (y..y + h).contains(&p[1])
    })
}
pub fn cancel_vibration() {}
/// No pointer capture on iOS: a touch has no cursor to hold.
pub fn capture_mouse(_capture: bool) -> bool {
    false
}
