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
pub fn send_inputs(inputs: &[Input]) -> Result<u32> {
    use std::mem;

    unsafe {
        // Calling C code
        let event_count = winuser::SendInput(
            inputs.len() as _,
            inputs.as_ptr() as _,
            mem::size_of::<winuser::INPUT>() as _,
        );

        if event_count == 0 {
            Err(crate::error::get_last_error())
        } else {
            Ok(event_count)
        }
    }
}
