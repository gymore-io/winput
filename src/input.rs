use crate::error::Result;

use winapi::um::winuser;

/// This structure is used by `send_input` to store information for synthesizing input
/// events such as keystrokes, mouse movement, and mouse clicks.
#[derive(Clone)]
#[repr(transparent)]
pub struct Input(winuser::INPUT);

/// Synthesizes keystrokes, mouse motions, and button clicks.
///
/// ## Returns
///
/// This function returns the number of events that were successfully inserted onto the
/// keyboard or mouse input stream.
///
/// In the case of no events inserted onto the keyboard or mouse input stream, an error is
/// returned.
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
/// ```rust
/// use winput::{Vk, Keylike};
///
/// let inputs = [
///     Vk::Control::press(),
///     Vk::A::press(),
///     'l'.press(),
///     'l'.release(),
///     'o'.press(),
///     'o'.release(),
///     'l'.press(),
///     'l'.release(),
///     Vk::A::release(),
///     Vk::Control::release(),
/// ];
///
/// winput::send_inputs(&inputs).unwrap();
/// ```
pub trait Keylike: Copy {
    /// Produces an `Input` that causes `self` to be pressed.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid value key.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use winput::Keylike;
    ///
    /// let input = 'A'.press();
    /// winput::send_inputs(&[input]);
    /// ```
    fn press(self) -> Input;

    /// Produces an `Input` that causes `self` to be released.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid value.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use winput::Keylike;
    ///
    /// let input = 'B'.release();
    /// winput::send_inputs(&[input]).unwrap();
    /// ```
    fn release(self) -> Input;

    /// Produces an `[Input; 2]` that causes `self` to be pressed then released.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid value.
    ///
    /// ## Example
    ///
    /// ```rust
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
