use crate::input::{send_inputs, Action, Button, Input};
use crate::vk::Vk;

/// A trait for objects that can be used as keys. For example a [`Vk`] or a `char` can be
/// used as a key.
///
/// ## Example
///
/// ```rust, ignore
/// use winput::Vk;
///
/// // Print `A`
/// winput::press(Vk::Shift);
/// winput::send(Vk::A);
/// winput::release(Vk::Shift);
///
/// // Do the same with one line
/// winput::send('A');
/// ```
///
/// [`Vk`]: enum.Vk.html
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
}

impl Keylike for char {
    #[inline(always)]
    fn produce_input(self, action: Action) -> Input {
        Input::from_char(self, action).expect("character above 0x0000ffff")
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

/// Synthesize an event that presses the key.
///
/// If the function fails to synthesize the input, no error is emited and the
/// function fails silently. If you wish to retreive an eventual error, use
/// `send_inputs` instead.
///
/// ## Panics
///
/// This function panics if `key` was not a valid key. For example, any `char` that
/// is above `0x0000ffff` cannot be turned into an `Input`.
///
/// ## Example
///
/// ```rust, ignore
/// winput::press('A').unwrap();
/// ```
#[inline]
pub fn press<K: Keylike>(key: K) {
    let input = key.produce_input(Action::Press);
    crate::input::send_inputs(&[input]);
}

/// Synthesizes an event that releases the key.
///
/// If the function fails to synthesize the input, no error is emited and the
/// function fails silently. If you wish to retreive an eventual error, use
/// `send_inputs` instead.
///
/// ## Panics
///
/// This function panics if `key` was not a valid key. For example, any `char` that
/// is above `0x0000ffff` cannot be turned into an `Input`.
///
/// ## Example
///
/// ```rust, ignore
/// winput::release('B').unwrap();
/// ```
#[inline(always)]
pub fn release<K: Keylike>(key: K) {
    let input = key.produce_input(Action::Release);
    crate::input::send_inputs(&[input]);
}

/// Synthesizes two events. One that presses the key, one that releases the key.
///
/// If the function fails to synthesize the input, no error is emited and the
/// function fails silently. If you wish to retreive an eventual error, use
/// `send_inputs` instead.
///
/// ## Panics
///
/// This function panics if `key` was not a valid value. For example, any `char` that
/// is above `0x0000ffff` cannot be turned into an `Input`.
///
/// ## Example
///
/// ```rust, ignore
/// winput::send('C').unwrap();
/// ```
#[inline(always)]
pub fn send<K: Keylike>(key: K) {
    let inputs = [
        key.produce_input(Action::Press),
        key.produce_input(Action::Release),
    ];

    crate::input::send_inputs(&inputs);
}

/// Synthesizes keystrokes according to the given iterator of keys.
///
/// Note that this function needs to allocate a buffer to store the inputs produced by the
/// given keys.
///
/// The function returns the number of inputs that were successfully inserted into the
/// keyboard input stream.
///
/// If no events were successfully sent, the input stream was already blocked by another
/// thread. You can use `winput::WindowsError::from_last_error` to retrieve additional
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

/// Synthesizes keystrokes following the given string reference.
///
/// Note that this function needs to allocate a buffer to store the inputs produced by
/// the characters.
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
