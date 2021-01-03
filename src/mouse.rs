use crate::error::WindowsError;
use crate::input::{send_inputs, Input, MouseMotion, WheelDirection};

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
    /// Mouse::set_position(50, 50).unwrap();
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

    /// Synthesizes a vertical scroll event.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Mouse;
    ///
    /// Mouse::scroll(1.0).unwrap();
    /// ```
    pub fn scroll(amount: f32) -> Result<(), WindowsError> {
        let input = Input::from_wheel(amount, WheelDirection::Vertical);
        let count = send_inputs(&[input]);

        if count == 0 {
            Err(WindowsError::from_last_error())
        } else {
            Ok(())
        }
    }

    /// Synthesizes a horizontal scroll event.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Mouse;
    ///
    /// Mouse::scrollh(1.0).unwrap();
    /// ```
    pub fn scrollh(amount: f32) -> Result<(), WindowsError> {
        let input = Input::from_wheel(amount, WheelDirection::Horizontal);
        let count = send_inputs(&[input]);

        if count == 0 {
            Err(WindowsError::from_last_error())
        } else {
            Ok(())
        }
    }

    /// Moves the mouse relativly to its current position, in screen coordinates.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Mouse;
    ///
    /// Mouse::move_relative(100, 50).unwrap();
    /// ```
    pub fn move_relative(dx: i32, dy: i32) -> Result<(), WindowsError> {
        let motion = MouseMotion::Relative { dx, dy };
        let input = Input::from_motion(motion);
        let count = send_inputs(&[input]);

        if count == 0 {
            Err(WindowsError::from_last_error())
        } else {
            Ok(())
        }
    }

    /// Moves the mouse using absolute normalized coordinates.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::Mouse;
    ///
    /// // Move the mouse in the center of the main monitor.
    /// Mouse::move_absolute(0.5, 0.5).unwrap();
    /// ```
    pub fn move_absolute(x: f32, y: f32) -> Result<(), WindowsError> {
        let motion = MouseMotion::Absolute {
            x,
            y,
            virtual_desk: false,
        };

        let input = Input::from_motion(motion);
        let count = send_inputs(&[input]);

        if count == 0 {
            Err(WindowsError::from_last_error())
        } else {
            Ok(())
        }
    }
}
