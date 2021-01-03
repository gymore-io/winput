use crate::error::{Error, Result};
use crate::vk::Vk;

use winapi::um::winuser;

/// Represents an action that can be taken on a key or button.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// The action of pressing the key.
    Press,
    /// The action of releasing the key.
    Release,
}

/// This structure is used by `send_input` to store information for synthesizing input
/// events such as keystrokes, mouse movement, and mouse clicks.
///
/// ## Example
///
/// ```rust, ignore
/// use winput::{Input, Action};
///
/// let input = Input::from_char('A', Action::Press);
/// winput::send_inputs(&[input]).unwrap();
/// ```
#[derive(Clone)]
#[repr(transparent)]
pub struct Input(winuser::INPUT);

impl Input {
    /// Creates an `Input` that causes the given action to be taken on the given
    /// character. This function fails with `Error::InvalidCharacter`if the given
    /// character is above `0x0000ffff`.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{Input, Action};
    ///
    /// let input = Input::from_char('A', Action::Press).unwrap();
    /// winput::send_inputs(&[input]).unwrap();
    /// ```
    pub fn from_char(c: char, action: Action) -> Result<Input> {
        let c_n = c as u32;
        if c_n > 0x0000ffff {
            return Err(Error::InvalidCharacter(c));
        }

        unsafe {
            let mut input: winuser::INPUT = std::mem::zeroed();
            input.type_ = winuser::INPUT_KEYBOARD;
            let ki = input.u.ki_mut();
            ki.wVk = 0; // ùust be 0 for a unicode event
            ki.wScan = c as u16;
            ki.dwFlags = match action {
                Action::Release => winuser::KEYEVENTF_KEYUP | winuser::KEYEVENTF_UNICODE,
                Action::Press => winuser::KEYEVENTF_UNICODE,
            };
            ki.time = 0; // let the system provide a time stamp

            Ok(Self(input))
        }
    }

    /// Creates an `Input` that causes the given action to be taken on the given
    /// Virtual-Key Code.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{Input, Action, Vk};
    ///
    /// let input = Input::from_vk(Vk::Enter, Action::Press);
    /// winput::send_inputs(&[input]).unwrap();
    /// ```
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

    /// Creates an `Input` that causes the given action to be taken on the given mouse
    /// button.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{Button, Action, Input};
    ///
    /// let input = Input::from_button(Button::Left, Action::Press);
    /// winput::send_inputs(&[input]).unwrap();
    /// ```
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

    /// Creates an `Input` that causes the mouse to move according to the given
    /// `MouseMotion`.
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
    /// winput::send_inputs(&[input]).unwrap();
    /// ```
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

    /// Creates an `Input` that causes the mouse wheel to rotate by the given amount and
    /// in the given direction.
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
    /// winput::send_inputs(&[input]).unwrap();
    /// ```
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
/// In the case of no events inserted onto the keyboard or mouse input stream, an error is
/// returned.
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
/// winput::send_inputs(&inputs).unwrap();
/// ```
pub fn send_inputs(inputs: impl AsRef<[Input]>) -> Result<u32> {
    use std::mem;

    unsafe {
        // Calling C code
        let event_count = winuser::SendInput(
            inputs.as_ref().len() as _,
            inputs.as_ref().as_ptr() as _,
            mem::size_of::<winuser::INPUT>() as _,
        );

        if event_count == 0 {
            Err(crate::error::get_last_error())
        } else {
            Ok(event_count)
        }
    }
}

/// A mouse button.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WheelDirection {
    Vertical,
    Horizontal,
}

/// A trait for objects that can be used as keys. For example `Vk` and `char` can be used
/// as keys.
///
/// ## Example
///
/// ```rust, ignore
/// use winput::{Vk, Keylike};
///
/// let inputs = [
///     Vk::Control.press(),
///     Vk::A.press(),
///     'l'.press(),
///     'l'.release(),
///     'o'.press(),
///     'o'.release(),
///     'l'.press(),
///     'l'.release(),
///     Vk::A.release(),
///     Vk::Control.release(),
/// ];
///
/// winput::send_inputs(&inputs).unwrap();
/// ```
pub trait Keylike: Copy {
    /// Produces an `Input` that causes the given action to be taken on `self`.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid key.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{Keylike, Action};
    ///
    /// let input = 'A'.produce_input(Action::Press);
    /// winput::send_inputs(&[input]).unwrap();
    /// ```
    fn produce_input(self, action: Action) -> Input;

    /// Produces an `Input` that causes `self` to be pressed.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid key.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Keylike;
    ///
    /// let input = 'A'.press();
    /// winput::send_inputs(&[input]);
    /// ```
    #[inline(always)]
    fn press(self) -> Input {
        self.produce_input(Action::Press)
    }

    /// Produces an `Input` that causes `self` to be released.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid key.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Keylike;
    ///
    /// let input = 'B'.release();
    /// winput::send_inputs(&[input]).unwrap();
    /// ```
    #[inline(always)]
    fn release(self) -> Input {
        self.produce_input(Action::Release)
    }

    /// Produces an `[Input; 2]` that causes `self` to be pressed then released.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid value.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Keylike;
    ///
    /// let input = 'C'.trigger();
    /// winput::send_inputs(&input).unwrap();
    /// ```
    #[inline(always)]
    fn trigger(self) -> [Input; 2] {
        [self.press(), self.release()]
    }
}

impl Keylike for char {
    #[inline(always)]
    fn produce_input(self, action: Action) -> Input {
        Input::from_char(self, action).expect("Invalid character")
    }
}

impl Keylike for Vk {
    #[inline(always)]
    fn produce_input(self, action: Action) -> Input {
        Input::from_vk(self, action)
    }
}

impl Keylike for Button {
    fn produce_input(self, action: Action) -> Input {
        Input::from_button(self, action)
    }
}

/// Synthesizes keystrokes according to the given iterator of keys. Note that this
/// function needs to allocate a buffer to store the inputs produced by the given keys.
///
/// The function returns the number of inputs that were successfully inserted into the
/// keyboard input stream.
///
/// ## Panics
///
/// This function panics if the buffer used to store the produced inputs fails to
/// allocate or if any of the given keys is unable to produce an `Input`.
///
/// ## Example
///
/// ```rust, ignore
/// use winput::Vk;
///
/// let keys = vec![ Vk::A, Vk::B, Vk::C ];
///
/// winput::send_keys(keys).unwrap();
/// ```
pub fn send_keys<I>(keys: I) -> Result<u32>
where
    I: IntoIterator,
    I::Item: Keylike,
{
    let iter = keys.into_iter();
    let mut buffer = Vec::with_capacity(iter.size_hint().0 * 2);

    for key in iter {
        buffer.extend_from_slice(&key.trigger());
    }

    send_inputs(&buffer)
}

/// Synthesizes keystrokes following the given string reference. Note that this function
/// needs to allocate a buffer to store the inputs produced by the characters.
///
/// The function returns the number of inputs that were successfully inserted into the
/// keyboard input stream.
///
/// ## Panics
///
/// This function panics if the buffer fails to allocate or if any of the given character
/// fails to produce an `Input`.
///
/// ## Example
///
/// ```rust, ignore
/// winput::send_str("Hello, world").unwrap();
/// ```
#[inline(always)]
pub fn send_str(s: &str) -> Result<u32> {
    send_keys(s.chars())
}
