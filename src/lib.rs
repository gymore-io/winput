mod vk;
pub use vk::Vk;

mod input;
pub use input::{
    send_inputs, send_keys, send_str, Action, Input, Keylike, MouseMotion, WheelDirection,
};
