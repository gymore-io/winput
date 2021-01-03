use crate::input::{send_inputs, Action, Button, Input};
use crate::vk::Vk;

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
/// winput::send_inputs(&inputs);
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
    /// winput::send_inputs(&[input]);
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
    /// winput::send_inputs(&[input]);
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
    /// winput::send_inputs(&input);
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
/// winput::send_str("Hello, world");
/// ```
#[inline(always)]
pub fn send_str(s: &str) -> u32 {
    send_keys(s.chars())
}
