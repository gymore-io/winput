mod error;
pub use error::WindowsError;

mod vk;
pub use vk::Vk;

mod input;
pub use input::{send_inputs, Action, Button, Input, MouseMotion, WheelDirection};

#[cfg(not(feature = "minimal"))]
mod keylike;
#[cfg(not(feature = "minimal"))]
pub use keylike::{press, release, send, send_keys, send_str, Keylike};

mod mouse;
pub use mouse::Mouse;

#[cfg(feature = "message_loop")]
pub mod message_loop;
