use crate::error::WindowsError;

use winapi::shared::windef;
use winapi::um::winuser;

/// A zero-sized structure that wraps functions related to the mouse.
pub struct Mouse;

impl Mouse {
    /// Retreive the current position of the mouse, in screen coordinates.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Mouse;
    ///
    /// println!("The mouse is at {:?}", Mouse::position());
    /// ```
    pub fn position() -> Result<(i32, i32), WindowsError> {
        unsafe {
            let mut point: windef::POINT = std::mem::zeroed();

            // Calling C code
            if winuser::GetCursorPos(&mut point) != 0 {
                Ok((point.x, point.y))
            } else {
                Err(WindowsError::from_last_error())
            }
        }
    }

    /// Sets the position of the mouse, in screen coordinates.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::{Vk, Mouse};
    ///
    /// while !Vk::Escape.is_down() {
    ///     if let Some((x, y)) = Mouse::position() {
    ///         Mouse::set_position(x + 10, y);
    ///     }
    /// }
    /// ```
    ///
    /// ## Note
    ///
    /// Note that the following code should be equivalent to using this function and
    /// provides more control. The only difference is that this function uses screen
    /// coordinates instead of normalized screen coordinates.
    ///
    /// ```rust, ignore
    /// use winput::{Input, MouseMotion};
    ///
    /// let motion = MouseMotion::Absolute { dx: 0.5, dy: 0.75, virtual_desk: false };
    /// let input = Input::from_motion(motion);
    /// winput::send_inputs(&[input]);
    /// ```
    pub fn set_position(x: i32, y: i32) -> Result<(), WindowsError> {
        unsafe {
            // Calling C code
            if winuser::SetCursorPos(x, y) == 0 {
                Err(WindowsError::from_last_error())
            } else {
                Ok(())
            }
        }
    }
}
