//! The iOS side of the platform calls the shared touch glue makes (`game/android_input.rs`), with
//! the same names as `android/platform.rs`. The system keyboard itself comes from winit's text
//! input (`game/ime.rs`); this module only records whether a tap asked for it.
use horizon_client::osk::Output;
use horizon_client::{android_gamepad::InputMode, android_runtime};
use std::sync::atomic::{AtomicBool, Ordering};

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

/// A tap on a text field asked for the keyboard (`show`), and the field has kept the focus since.
/// iOS shows the keyboard as soon as text input is allowed, so `game/ime.rs` allows it only then:
/// a screen that focuses its search box by itself (the world list) leaves the keyboard down.
static REQUESTED: AtomicBool = AtomicBool::new(false);

/// Whether the keyboard was asked for.
pub fn active() -> bool {
    REQUESTED.load(Ordering::Relaxed)
}
pub fn show(_text: &str) {
    REQUESTED.store(true, Ordering::Relaxed);
}
pub fn show_multiline(text: &str) {
    show(text);
}
pub fn hide() {
    REQUESTED.store(false, Ordering::Relaxed);
}
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
