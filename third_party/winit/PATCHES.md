# Winit Android input patch

Upstream: winit 0.30.13 from crates.io, original Apache-2.0 license retained.

Android-only mouse hover/buttons/wheel/relative events and modifier snapshots
are forwarded as normal winit events. Android touch and absolute mouse coordinates use the same safe-area/aspect
rectangle as presentation, including resolution scaling. New touches outside
that rectangle are ignored; existing gestures still receive their releases.
Desktop backends retain their upstream behaviour. A stock Android NativeActivity host is used for full
mouse input; no project Java/Kotlin host is included.

# Winit iOS window-area patch

On iOS, `Window::inner_size` and `inner_position` return the whole view (the same size
`WindowEvent::Resized` carries and the `CAMetalLayer` covers) instead of the safe area, so a
renderer sized from either draws edge to edge. The safe area is
`WindowExtIOS::safe_area_insets()`: physical left, top, right and bottom insets. Other backends are
unchanged.

The iOS on-screen keyboard's characters arrive with the key code of the US-layout key that types
them (letters, digits, punctuation, Space), and Return as the Enter key without text, instead of an
unidentified key: code that reads key codes, like a chat box, then takes the typed text. The text
itself is unchanged.
