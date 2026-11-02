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

/// Where the on-screen keyboard is, from UIKit's keyboard notifications, which winit does not pass
/// on: the game lifts the chat above it.
#[cfg(target_os = "ios")]
pub mod keyboard {
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObjectProtocol, ProtocolObject};
    use objc2_core_foundation::CGRect;
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSValue, ns_string};
    use std::ptr::NonNull;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// The keyboard's top edge in points from the top of the screen; infinity while hidden.
    static TOP: AtomicU64 = AtomicU64::new(0x7FF0_0000_0000_0000);
    /// The observer, kept for the process's lifetime (pointer, so `Send`/`Sync` hold).
    static OBSERVER: OnceLock<usize> = OnceLock::new();

    /// Starts following the keyboard's frame. Call once, on the main thread.
    pub fn observe() {
        OBSERVER.get_or_init(|| {
            let block = block2::RcBlock::new(|notification: NonNull<NSNotification>| {
                // Safety: UIKit passes a live notification for the duration of the call.
                let notification = unsafe { notification.as_ref() };
                let Some(info) = notification.userInfo() else {
                    return;
                };
                let key: &AnyObject = ns_string!("UIKeyboardFrameEndUserInfoKey");
                let Some(value) = info.objectForKey(key) else {
                    return;
                };
                // Safety: UIKit documents this key's value as an NSValue holding the end frame.
                let value: &NSValue = unsafe { &*(Retained::as_ptr(&value) as *const NSValue) };
                // Safety: `CGRectValue` (UIKit's NSValue addition) on an NSValue holding a CGRect.
                let frame: CGRect = unsafe { objc2::msg_send![value, CGRectValue] };
                TOP.store(frame.origin.y.to_bits(), Ordering::Relaxed);
            });
            // Safety: the default center and a block that only reads its argument and stores an
            // atomic; delivered on the posting (main) thread, as no queue is given.
            let observer: Retained<ProtocolObject<dyn NSObjectProtocol>> = unsafe {
                NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                    Some(ns_string!("UIKeyboardWillChangeFrameNotification")),
                    None,
                    None,
                    &block,
                )
            };
            Retained::into_raw(observer) as usize
        });
    }

    /// How far the keyboard reaches up from the bottom of a view `view_height` points tall, in
    /// points; 0 while it is hidden.
    pub fn covered(view_height: f64) -> f64 {
        let top = f64::from_bits(TOP.load(Ordering::Relaxed));
        if top.is_finite() {
            (view_height - top).clamp(0.0, view_height)
        } else {
            0.0
        }
    }
}

/// Safari and the clipboard, for the Microsoft sign-in's link and code. Main thread only.
#[cfg(target_os = "ios")]
pub mod links {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::{NSDictionary, NSString, NSURL};

    fn class(name: &std::ffi::CStr) -> Option<&'static AnyClass> {
        AnyClass::get(name)
    }

    /// Opens `url` in its app (Safari for a web page). False when it could not be asked.
    pub fn open_url(url: &str) -> bool {
        let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) else {
            return false;
        };
        let Some(application) = class(c"UIApplication") else {
            return false;
        };
        let options = NSDictionary::<AnyObject, AnyObject>::new();
        // Safety: UIApplication's shared instance and `openURL:options:completionHandler:` on the
        // main thread, with a valid URL, an empty options dictionary and no completion block.
        unsafe {
            let shared: *mut AnyObject = msg_send![application, sharedApplication];
            if shared.is_null() {
                return false;
            }
            let none: *mut AnyObject = std::ptr::null_mut();
            let _: () =
                msg_send![shared, openURL: &*url, options: &*options, completionHandler: none];
        }
        true
    }

    /// Puts `text` on the system clipboard.
    pub fn copy_text(text: &str) -> bool {
        let Some(pasteboard) = class(c"UIPasteboard") else {
            return false;
        };
        // Safety: UIPasteboard's general pasteboard and `setString:` with a valid string.
        unsafe {
            let general: *mut AnyObject = msg_send![pasteboard, generalPasteboard];
            if general.is_null() {
                return false;
            }
            let _: () = msg_send![general, setString: &*NSString::from_str(text)];
        }
        true
    }
}

/// SRV records through the system resolver: an iOS app can't read `/etc/resolv.conf`, which
/// Horizon's own DNS client asks for its name servers, so `hypixel.net`-style addresses (an SRV
/// record pointing at the game server) went to the web server and timed out. The system's answer
/// follows the active network, VPN and private DNS, like Android's (`android/dns.rs`).
#[cfg(target_os = "ios")]
pub mod dns {
    use std::ffi::{CString, c_char, c_int};

    #[link(name = "resolv")]
    unsafe extern "C" {
        /// `res_query` of `<resolv.h>`, which names it `res_9_query` on Apple platforms.
        fn res_9_query(
            name: *const c_char,
            class: c_int,
            kind: c_int,
            answer: *mut u8,
            length: c_int,
        ) -> c_int;
    }

    /// The complete DNS reply to an SRV question for `name`, for horizon-protocol to parse.
    pub fn query_srv(name: &str) -> Option<Vec<u8>> {
        const CLASS_IN: c_int = 1;
        const TYPE_SRV: c_int = 33;
        let name = CString::new(name).ok()?;
        let mut answer = vec![0u8; 8192];
        // Safety: a NUL-terminated name and a buffer of the length given; the resolver writes at
        // most that many bytes and returns how many, or -1.
        let length = unsafe {
            res_9_query(
                name.as_ptr(),
                CLASS_IN,
                TYPE_SRV,
                answer.as_mut_ptr(),
                answer.len() as c_int,
            )
        };
        let length = usize::try_from(length).ok().filter(|n| *n > 0)?;
        answer.truncate(length.min(answer.len()));
        Some(answer)
    }
}
