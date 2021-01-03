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
    /// ```rust
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
    /// ```rust
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
