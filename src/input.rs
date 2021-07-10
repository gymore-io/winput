use crate::vk::Vk;

use winapi::um::winuser;

/// This structure is used by [`send_inputs`] to store information for synthesizing input
/// events such as keystrokes, mouse movement, and mouse clicks.
///
/// ## Example
///
/// ```rust, ignore
/// use winput::{Input, Action};
///
/// let input = Input::from_char('A', Action::Press);
/// winput::send_inputs(&[input]);
/// ```
///
/// [`send_inputs`]: fn.send_inputs.html
#[derive(Clone)]
#[repr(transparent)]
pub struct Input(winuser::INPUT);

impl Input {
    /// Creates an [`Input`] that causes the given action to be taken on the given
    /// character. If the given character is above `0x0000ffff`, `None` is returned.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{Input, Action};
    ///
    /// let input = Input::from_char('A', Action::Press).unwrap();
    /// winput::send_inputs(&[input]);
    /// ```
    ///
    /// [`Input`]: struct.Input.html
    pub fn from_char(c: char, action: Action) -> Option<Input> {
        let c_n = c as u32;
        if c_n > 0x0000ffff {
            return None;
        }

        unsafe {
            let mut input: winuser::INPUT = std::mem::zeroed();
            input.type_ = winuser::INPUT_KEYBOARD;
            let ki = input.u.ki_mut();
            ki.wVk = 0; // must be 0 for a unicode event
            ki.wScan = c as u16;
            ki.dwFlags = match action {
                Action::Release => winuser::KEYEVENTF_KEYUP | winuser::KEYEVENTF_UNICODE,
                Action::Press => winuser::KEYEVENTF_UNICODE,
            };
            ki.time = 0; // let the system provide a time stamp

            Some(Self(input))
        }
    }

    /// Creates an [`Input`] that causes the given action to be taken on the given
    /// Virtual-Key Code.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{Input, Action, Vk};
    ///
    /// let input = Input::from_vk(Vk::Enter, Action::Press);
    /// winput::send_inputs(&[input]);
    /// ```
    ///
    /// [`Input`]: struct.Input.html
    pub fn from_vk(vk: Vk, action: Action) -> Input {
        unsafe {
            let mut input: winuser::INPUT = std::mem::zeroed();
            input.type_ = winuser::INPUT_KEYBOARD;
            let ki = input.u.ki_mut();
            ki.wVk = vk as u16;
            ki.wScan = 0; // we are using the Virtual-Key Code
            ki.dwFlags = match action {
                Action::Release => winuser::KEYEVENTF_KEYUP,
                Action::Press => 0,
            };
            ki.time = 0; // let the system provide a time stamp

            Self(input)
        }
    }

    /// Creates an [`Input`] that causes the given action to be taken on the given mouse
    /// button.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{Button, Action, Input};
    ///
    /// let input = Input::from_button(Button::Left, Action::Press);
    /// winput::send_inputs(&[input]);
    /// ```
    ///
    /// [`Input`]: struct.Input.html
    pub fn from_button(button: Button, action: Action) -> Input {
        unsafe {
            let mut input: winuser::INPUT = std::mem::zeroed();
            input.type_ = winuser::INPUT_MOUSE;
            let mi = input.u.mi_mut();
            mi.dx = 0; // the mouse is not going to move
            mi.dy = 0;
            mi.mouseData = match button {
                Button::X1 => 1,
                Button::X2 => 2,
                _ => 0,
            };
            mi.dwFlags = match button {
                Button::Left => match action {
                    Action::Press => winuser::MOUSEEVENTF_LEFTDOWN,
                    Action::Release => winuser::MOUSEEVENTF_LEFTUP,
                },
                Button::Right => match action {
                    Action::Press => winuser::MOUSEEVENTF_RIGHTDOWN,
                    Action::Release => winuser::MOUSEEVENTF_RIGHTUP,
                },
                Button::Middle => match action {
                    Action::Press => winuser::MOUSEEVENTF_MIDDLEDOWN,
                    Action::Release => winuser::MOUSEEVENTF_MIDDLEUP,
                },
                Button::X1 | Button::X2 => match action {
                    Action::Press => winuser::MOUSEEVENTF_XDOWN,
                    Action::Release => winuser::MOUSEEVENTF_XUP,
                },
            };

            mi.time = 0; // let the system provide a time stamp
            mi.dwExtraInfo = 0; // no extra information

            Self(input)
        }
    }

    /// Creates an [`Input`] that causes the mouse to move according to the given
    /// [`MouseMotion`].
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{MouseMotion, Input};
    ///
    /// let motion = MouseMotion::Relative {
    ///     dx: 100, // 100 pixels right
    ///     dy: 50, // 50 pixels down
    /// };
    ///
    /// let input = Input::from_motion(motion);
    ///
    /// winput::send_inputs(&[input]);
    /// ```
    ///
    /// [`Input`]: struct.Input.html
    /// [`MouseMotion`]: enum.MouseMotion.html
    pub fn from_motion(motion: MouseMotion) -> Self {
        unsafe {
            let mut input: winuser::INPUT = std::mem::zeroed();
            input.type_ = winuser::INPUT_MOUSE;
            let mi = input.u.mi_mut();

            mi.mouseData = 0; // no additional data

            // in any case, the mouse is goign to move
            mi.dwFlags = winuser::MOUSEEVENTF_MOVE;

            match motion {
                MouseMotion::Relative { dx, dy } => {
                    mi.dx = dx;
                    mi.dy = dy;
                }
                MouseMotion::Absolute { x, y, virtual_desk } => {
                    const NORMAL_FACTOR: f32 = 65535.0;

                    mi.dx = (x * NORMAL_FACTOR) as i32;
                    mi.dy = (y * NORMAL_FACTOR) as i32;

                    if virtual_desk {
                        mi.dwFlags |= winuser::MOUSEEVENTF_VIRTUALDESK;
                    }

                    mi.dwFlags |= winuser::MOUSEEVENTF_ABSOLUTE;
                }
            }

            mi.time = 0; // let the system provide a time stamp
            mi.dwExtraInfo = 0; // no extra information

            Self(input)
        }
    }

    /// Creates an [`Input`] that causes the mouse wheel to rotate by the given amount and
    /// in the given [`WheelDirection`].
    ///
    /// When the given direction is vertical, a positive motion means the wheel rotates
    /// forward, away from the user; a negative value means the wheel rotates backward,
    /// toward the user.
    ///
    /// When the given direction is horizontal, a positive motion means the wheel rotates
    /// to the right; a negative value means the wheel rotates to the left.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{WheelDirection, Input};
    ///
    /// let input = Input::from_wheel(100.0, WheelDirection::Vertical);
    /// winput::send_inputs(&[input]);
    /// ```
    ///
    /// [`Input`]: struct.Input.html
    /// [`WheelDirection`]: enum.WheelDirection.html
    pub fn from_wheel(motion: f32, direction: WheelDirection) -> Self {
        unsafe {
            let mut input: winuser::INPUT = std::mem::zeroed();
            input.type_ = winuser::INPUT_MOUSE;
            let mi = input.u.mi_mut();
            mi.dx = 0; // there is no mouse movement
            mi.dy = 0;

            const MOUSE_DELTA: f32 = 120.0;
            mi.mouseData = (motion * MOUSE_DELTA) as i32 as u32;

            mi.dwFlags = match direction {
                WheelDirection::Vertical => winuser::MOUSEEVENTF_WHEEL,
                WheelDirection::Horizontal => winuser::MOUSEEVENTF_HWHEEL,
            };

            mi.time = 0; // let the system provide a time stamp
            mi.dwExtraInfo = 0; // no extra information

            Self(input)
        }
    }
}

/// Synthesizes keystrokes, mouse motions, and button clicks.
///
/// ## Returns
///
/// This function returns the number of events that were successfully inserted onto the
/// keyboard or mouse input stream.
///
/// If no events were successfully sent, the input stream was already blocked by another
/// thread. You can use [`winput::WindowsError::from_last_error`] to retrieve additional
/// information about this function failing to send events.
///
/// ## Example
///
/// ```rust, ignore
/// use winput::{Vk, Input, Action};
///
/// let inputs = [
///     Input::from_vk(Vk::Shift, Action::Press),
///     Input::from_vk(Vk::A, Action::Press),
///     Input::from_vk(Vk::A, Action::Release),
///     Input::from_vk(Vk::Shift, Action::Release),
/// ];
///
/// winput::send_inputs(&inputs);
/// ```
///
/// [`winput::WindowsError::from_last_error`]: struct.WindowsError.html#method.from_last_error
pub fn send_inputs(inputs: impl AsRef<[Input]>) -> u32 {
    use std::mem;

    // Calling C code
    unsafe {
        winuser::SendInput(
            inputs.as_ref().len() as _,
            inputs.as_ref().as_ptr() as _,
            mem::size_of::<winuser::INPUT>() as _,
        )
    }
}

/// Represents an action that can be taken on a key or button.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Action {
    /// The action of pressing the key.
    Press,
    /// The action of releasing the key.
    Release,
}

impl Action {
    /// Creates a new [`Action`] from the given `bool`.
    ///
    /// * If `is_press` is `true`, `Action::Press` is returned.
    /// * If `is_press` is `false`, `Action::Release` is returned.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use winput::Action;
    ///
    /// assert_eq!(Action::from_press(true), Action::Press);
    /// assert_eq!(Action::from_press(false), Action::Release);
    /// ```
    ///
    /// [`Action`]: enum.Action.html
    pub fn from_press(is_press: bool) -> Self {
        if is_press {
            Self::Press
        } else {
            Self::Release
        }
    }

    /// Creates a new [`Action`] from the given `bool`.
    ///
    /// * If `is_release` is `true`, `Action::Release` is returned.
    /// * If `is_release` is `false`, `Action::Press` is returned.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use winput::Action;
    ///
    /// assert_eq!(Action::from_release(true), Action::Release);
    /// assert_eq!(Action::from_release(false), Action::Press);
    /// ```
    ///
    /// [`Action`]: enum.Action.html
    pub fn from_release(is_release: bool) -> Self {
        if is_release {
            Self::Release
        } else {
            Self::Press
        }
    }
}

/// A mouse button.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Button {
    /// The left mouse button.
    Left,
    /// The right mouse button.
    Right,
    /// The middle mouse button.
    Middle,
    /// The X1 mouse button.
    X1,
    /// The X2 mouse button.
    X2,
}

/// Describes a mouse motion.
#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseMotion {
    /// Describes a relative mouse motion, in pixels.
    ///
    /// Relative mouse motion is subject to the effects of the mouse speed and the
    /// two-mouse threshold values. A user sets these three values with the Pointer Speed
    /// slider of the Control Panel's Mouse Properties sheet.
    Relative {
        /// The number of pixels the mouse should move, on the horizontal axis.
        dx: i32,
        /// The number of pixels the mouse should move, on the vertical axis. A positive
        /// value makes the mouse go down.
        dy: i32,
    },
    /// Describes an absolute mouse motion, in normalized coordinates.
    Absolute {
        /// The normalized position of the mouse on the horizontal axis. A value of
        /// `0.0` maps to the left of the screen and a value of `1.0` maps to the
        /// right of the screen.
        x: f32,
        /// The normalized position of the mouse on the vertical axis. A value of `0.0`
        /// maps to the top of the screen and a value of `1.0` maps to the bottom on the
        /// screen.
        y: f32,
        /// Whether the given normalized coordinates should map to the entier virtual
        /// desktop (if multiple monitors are used, for example).
        virtual_desk: bool,
    },
}

/// Describes the direction of a mouse wheel.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WheelDirection {
    Vertical,
    Horizontal,
}
