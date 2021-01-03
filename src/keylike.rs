use crate::error::WindowsError;
use crate::input::{send_inputs, Action, Button, Input};
use crate::vk::Vk;

/// A trait for objects that can be used as keys. For example a `Vk` and a `char` can be
/// used as a key.
///
/// ## Example
///
/// ```rust, ignore
/// use winput::{Vk, Keylike};
///
/// Vk::Control.press().unwrap();
/// Vk::A.trigger().unwrap();
/// Vk::Control.release().unwrap();
/// ```
pub trait Keylike: Copy {
    /// Produces an `Input` that causes the given action to be taken on `self`.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid key. For example, any `char` that
    /// is above `0x0000ffff` cannot be turned into an `Input`.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{Keylike, Action};
    ///
    /// let input = 'A'.produce_input(Action::Press);
    /// winput::send_inputs(&[input]);
    /// ```
    fn produce_input(self, action: Action) -> Input;

    /// Synthesize an event that presses the key.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid key. For example, any `char` that
    /// is above `0x0000ffff` cannot be turned into an `Input`.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Keylike;
    ///
    /// 'A'.press().unwrap();
    /// ```
    #[inline(always)]
    fn press(self) -> Result<(), WindowsError> {
        let input = self.produce_input(Action::Press);
        let count = crate::input::send_inputs(&[input]);

        if count == 1 {
            Ok(())
        } else {
            Err(WindowsError::from_last_error())
        }
    }

    /// Synthesizes an event that releases the key.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid key. For example, any `char` that
    /// is above `0x0000ffff` cannot be turned into an `Input`.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Keylike;
    ///
    /// 'B'.release().unwrap();
    /// ```
    #[inline(always)]
    fn release(self) -> Result<(), WindowsError> {
        let input = self.produce_input(Action::Release);
        let count = crate::input::send_inputs(&[input]);

        if count == 1 {
            Ok(())
        } else {
            Err(WindowsError::from_last_error())
        }
    }

    /// Synthesizes two events. One that presses the key, one that releases the key.
    ///
    /// ## Panics
    ///
    /// This function panics if `self` was not a valid value. For example, any `char` that
    /// is above `0x0000ffff` cannot be turned into an `Input`.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Keylike;
    ///
    /// 'C'.trigger().unwrap();
    /// ```
    #[inline(always)]
    fn send(self) -> Result<(), WindowsError> {
        let inputs = [
            self.produce_input(Action::Press),
            self.produce_input(Action::Release),
        ];

        let count = crate::input::send_inputs(&inputs);

        if count == 0 {
            Err(WindowsError::from_last_error())
        } else {
            Ok(())
        }
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
/// If no events were successfully sent, the input stream was already blocked by another
/// thread. You can use `winput::WindowsError::from_last_error` to retreive additional
/// information about this function failing to send events.
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
/// winput::send_keys(keys);
/// ```
pub fn send_keys<I>(keys: I) -> u32
where
    I: IntoIterator,
    I::Item: Keylike,
{
    let iter = keys.into_iter();
    let mut buffer = Vec::with_capacity(iter.size_hint().0 * 2);

    for key in iter {
        buffer.push(key.produce_input(Action::Press));
        buffer.push(key.produce_input(Action::Release));
    }

    send_inputs(&buffer)
}

/// Synthesizes keystrokes following the given string reference. Note that this function
/// needs to allocate a buffer to store the inputs produced by the characters.
///
/// The function returns the number of inputs that were successfully inserted into the
/// keyboard input stream.
///
/// If no events were successfully sent, the input stream was already blocked by another
/// thread. You can use `winput::WindowsError::from_last_error` to retreive additional
/// information about this function failing to send events.
///
/// ## Panics
///
/// This function panics if the buffer fails to allocate or if any of the given character
/// fails to produce an `Input`.
///
/// ## Example
///
/// ```rust, ignore
/// winput::send_str("Hello, world");
/// ```
#[inline(always)]
pub fn send_str(s: &str) -> u32 {
    send_keys(s.chars())
}
