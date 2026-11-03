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

/// The gyroscope through CoreMotion, for gyro aiming (Options → Controls → Touch Controls). Read
/// once a frame on the main thread; updates stop while gyro aiming is off, so the sensor draws no
/// power then.
#[cfg(target_os = "ios")]
pub mod motion {
    use objc2::encode::{Encode, Encoding};
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, Bool};
    use std::cell::RefCell;

    #[link(name = "CoreMotion", kind = "framework")]
    unsafe extern "C" {}

    /// `CMRotationRate`: radians a second around the device's x (right), y (up, in portrait)
    /// and z (out of the screen) axes, counter-clockwise positive, as Android's gyroscope.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RotationRate {
        x: f64,
        y: f64,
        z: f64,
    }
    // Safety: the layout and encoding of CoreMotion's `{?=ddd}` struct.
    unsafe impl Encode for RotationRate {
        const ENCODING: Encoding =
            Encoding::Struct("?", &[Encoding::Double, Encoding::Double, Encoding::Double]);
    }

    thread_local! {
        static MANAGER: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
    }

    /// The device's rotation rate now, starting the sensor when `active` and stopping it when
    /// not; `None` without a gyroscope or before its first sample.
    pub fn rate(active: bool) -> Option<[f32; 3]> {
        MANAGER.with(|slot| {
            let mut slot = slot.borrow_mut();
            if !active {
                if let Some(manager) = slot.take() {
                    // Safety: a CMMotionManager this module made, on the main thread.
                    unsafe {
                        let _: () = msg_send![&*manager, stopDeviceMotionUpdates];
                    }
                }
                return None;
            }
            if slot.is_none() {
                let class = AnyClass::get(c"CMMotionManager")?;
                // Safety: `+new` of CMMotionManager, then its documented setup on the main
                // thread: whether device motion exists, its interval, and pull-mode updates.
                let manager: Retained<AnyObject> = unsafe { msg_send![class, new] };
                let available: Bool = unsafe { msg_send![&*manager, isDeviceMotionAvailable] };
                if !available.as_bool() {
                    return None;
                }
                unsafe {
                    let _: () = msg_send![&*manager, setDeviceMotionUpdateInterval: 1.0f64 / 60.0];
                    let _: () = msg_send![&*manager, startDeviceMotionUpdates];
                }
                *slot = Some(manager);
            }
            let manager = slot.as_ref()?;
            // Safety: the latest CMDeviceMotion (nil before the first sample) and its
            // `rotationRate`, the bias-corrected gyroscope reading.
            let rate: RotationRate = unsafe {
                let motion: *mut AnyObject = msg_send![&**manager, deviceMotion];
                if motion.is_null() {
                    return None;
                }
                msg_send![motion, rotationRate]
            };
            Some([rate.x as f32, rate.y as f32, rate.z as f32])
        })
    }

    /// How the interface is turned, as Android's `Display.getRotation` counts it: 0 portrait,
    /// 1 landscape with the home side on the right, 2 upside down, 3 landscape with it on the left.
    pub fn interface_rotation() -> i32 {
        let Some(application) = AnyClass::get(c"UIApplication") else {
            return 1;
        };
        // Safety: the shared application's connected scenes, on the main thread; a window
        // scene's `interfaceOrientation` (UIInterfaceOrientation, an NSInteger).
        let orientation: isize = unsafe {
            let shared: *mut AnyObject = msg_send![application, sharedApplication];
            if shared.is_null() {
                return 1;
            }
            let scenes: *mut AnyObject = msg_send![shared, connectedScenes];
            if scenes.is_null() {
                return 1;
            }
            let scene: *mut AnyObject = msg_send![scenes, anyObject];
            if scene.is_null() {
                return 1;
            }
            msg_send![scene, interfaceOrientation]
        };
        match orientation {
            1 => 0, // UIInterfaceOrientationPortrait
            2 => 2, // UIInterfaceOrientationPortraitUpsideDown
            4 => 3, // UIInterfaceOrientationLandscapeLeft: home side on the left
            _ => 1, // UIInterfaceOrientationLandscapeRight, or unknown: the game's landscape
        }
    }
}
