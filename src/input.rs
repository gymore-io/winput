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
