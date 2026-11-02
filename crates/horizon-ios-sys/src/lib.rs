//! The iOS system calls that `horizon-client`, which forbids unsafe code, can't make itself
//! (docs/ios.md). On other platforms the crate is empty.

/// Controllers through Apple's GameController framework: Xbox, PlayStation, MFi and Switch Pro
/// controllers paired over Bluetooth. gilrs has no iOS backend.
#[cfg(target_os = "ios")]
pub mod gamepad {
    use horizon_client_input::gamepad::{Button, PadState};
    use objc2_game_controller::{GCController, GCControllerButtonInput, GCExtendedGamepad};

    /// The controller in use (the one last touched, else the first connected): its buttons,
    /// sticks and triggers now, or `None` without one. Called on the main thread, once a frame.
    pub fn state() -> Option<PadState> {
        // Safety: class methods of GCController, called on the main thread as the framework
        // requires; the returned objects are retained for the duration of this call.
        let controller = unsafe { GCController::current() }
            .or_else(|| unsafe { GCController::controllers() }.firstObject())?;
        // Safety: as above, a property read of a live controller.
        let pad = unsafe { controller.extendedGamepad() }?;
        Some(read(&pad))
    }

    fn read(pad: &GCExtendedGamepad) -> PadState {
        // Safety (every block below): property reads of a live, retained GCExtendedGamepad and of
        // the input elements it owns; none of them mutates state.
        let pressed = |b: &GCControllerButtonInput| unsafe { b.isPressed() };
        let mut state = PadState::default();
        unsafe {
            let dpad = pad.dpad();
            for (button, input) in [
                (Button::South, pad.buttonA()),
                (Button::East, pad.buttonB()),
                (Button::West, pad.buttonX()),
                (Button::North, pad.buttonY()),
                (Button::LeftBumper, pad.leftShoulder()),
                (Button::RightBumper, pad.rightShoulder()),
                (Button::Start, pad.buttonMenu()),
                (Button::DPadUp, dpad.up()),
                (Button::DPadDown, dpad.down()),
                (Button::DPadLeft, dpad.left()),
                (Button::DPadRight, dpad.right()),
            ] {
                state.set(button, pressed(&input));
            }
            for (button, input) in [
                (Button::Select, pad.buttonOptions()),
                (Button::LeftStick, pad.leftThumbstickButton()),
                (Button::RightStick, pad.rightThumbstickButton()),
            ] {
                state.set(button, input.is_some_and(|b| pressed(&b)));
            }
            let stick = |s: &objc2_game_controller::GCControllerDirectionPad| {
                [s.xAxis().value(), s.yAxis().value()]
            };
            // Up is positive on both GameController's sticks and PadState's.
            state.left = stick(&pad.leftThumbstick());
            state.right = stick(&pad.rightThumbstick());
            state.triggers = [pad.leftTrigger().value(), pad.rightTrigger().value()];
        }
        state
    }
}
