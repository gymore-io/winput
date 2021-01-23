use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use winapi::um::winuser;

use crate::input::{Action, Button};
use crate::vk::Vk;

/// The current state of the message loop.
///
/// * 0 -> The message loop is not active.
/// * 1 -> The `start` function has been called.
///      The message loop is now starting.
/// * 2 -> The message loop has successfully started.
/// * 3 -> The message loop is now exiting.
static STATE: AtomicU8 = AtomicU8::new(0);

// Those values are always initialized if `STARTED` is `true`.
// `SENDER` must only be used on the message loop's thread.
static mut SENDER: MaybeUninit<mpsc::Sender<Event>> = MaybeUninit::uninit();

/// Blocks the calling thread (with a spin-lock) until `STATE` has the given value.
#[inline(always)]
fn block_until_state_is(val: u8) {
    while STATE.load(Ordering::SeqCst) != val {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if code >= 0 {
        // SAFETY: The given `l_param` is pointer to a valid `KBDLLHOOKSTRUCT`.
        let kbd_hook_struct = &*(l_param as *const winuser::KBDLLHOOKSTRUCT);

        let event = Event::Keyboard {
            vk: Vk::from_u8(kbd_hook_struct.vkCode as _),
            scan_code: kbd_hook_struct.scanCode,
            action: Action::from_release(kbd_hook_struct.flags == winuser::LLKHF_UP),
        };

        // SAFETY: If this function was called, then the message loop was started
        // and the `SENDER` is thus initialized.
        //
        // `SENDER` must only be used on the message loop's thread. This callback function
        // is called on this thread.
        //
        // For this reason, we do have an exclusive reference to the `gloval_sender` field.
        (&*SENDER.as_ptr()).send(event).unwrap();
    }

    winuser::CallNextHookEx(ptr::null_mut(), code, w_param, l_param)
}

unsafe extern "system" fn low_level_mouse_proc(
    code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if code >= 0 {
        // SAFETY: The given `l_param` is pointer to a valid `MSLLHOOKSTRUCT`.
        let ms_hook_struct = &*(l_param as *const winuser::MSLLHOOKSTRUCT);

        let event = match w_param as u32 {
            winuser::WM_LBUTTONDOWN => Event::MouseButton {
                x: ms_hook_struct.pt.x,
                y: ms_hook_struct.pt.y,
                action: Action::Press,
                button: Button::Left,
            },

            winuser::WM_LBUTTONUP => Event::MouseButton {
                x: ms_hook_struct.pt.x,
                y: ms_hook_struct.pt.y,
                action: Action::Release,
                button: Button::Right,
            },

            winuser::WM_RBUTTONDOWN => Event::MouseButton {
                x: ms_hook_struct.pt.x,
                y: ms_hook_struct.pt.y,
                action: Action::Press,
                button: Button::Right,
            },

            winuser::WM_RBUTTONUP => Event::MouseButton {
                x: ms_hook_struct.pt.x,
                y: ms_hook_struct.pt.y,
                action: Action::Release,
                button: Button::Right,
            },

            winuser::WM_XBUTTONDOWN => Event::MouseButton {
                x: ms_hook_struct.pt.x,
                y: ms_hook_struct.pt.y,
                action: Action::Press,
                // Only the high-order word is used to store the button.
                button: match (ms_hook_struct.mouseData >> 4) as u16 {
                    winuser::XBUTTON1 => Button::X1,
                    winuser::XBUTTON2 => Button::X2,
                    _ => unreachable!("Invalid button: {}", ms_hook_struct.mouseData),
                },
            },

            winuser::WM_XBUTTONUP => Event::MouseButton {
                x: ms_hook_struct.pt.x,
                y: ms_hook_struct.pt.y,
                action: Action::Release,
                // Only the high-order word is used to store the button.
                button: match (ms_hook_struct.mouseData >> 4) as u16 {
                    winuser::XBUTTON1 => Button::X1,
                    winuser::XBUTTON2 => Button::X2,
                    _ => unreachable!("Invalid button: {}", ms_hook_struct.mouseData),
                },
            },

            winuser::WM_MOUSEMOVE => Event::MouseMove {
                x: ms_hook_struct.pt.x,
                y: ms_hook_struct.pt.y,
            },

            winuser::WM_MOUSEWHEEL => Event::MouseWheel {
                x: ms_hook_struct.pt.x,
                y: ms_hook_struct.pt.y,
                // Only the high-order word is used to store the delta.
                delta: (ms_hook_struct.mouseData >> 4) as f32 / 120.0,
            },

            _ => unreachable!("Invalid message"),
        };

        // SAFETY: See `low_level_keyboard_proc`
        (&*SENDER.as_ptr()).send(event).expect("Channel poisoned");
    }

    winuser::CallNextHookEx(ptr::null_mut(), code, w_param, l_param)
}

/// Starts the message loop on a new thread.
///
/// ## Panics
///
/// This function panics if the message loop is already active.
///
/// ## Example
///
/// ```rust, ignore
/// use winput::message_loop;
///
/// let receiver = message_loop::start();
///
/// loop {
///     println!("{:?}", receiver.next_event());
/// }
/// ```
pub fn start() -> EventReceiver {
    assert_eq!(
        STATE.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst),
        Ok(0),
        "The message loop was already active"
    );

    // The message loop is now starting.
    // This channel is used to receive the messages of the message loop.
    let (s, r) = mpsc::channel();

    // We have to initialize `SENDER`.
    unsafe { SENDER = MaybeUninit::new(s) };

    std::thread::spawn(|| {
        println!("Message loop starting");

        unsafe {
            // Install the hooks

            let keyboard_hook = winuser::SetWindowsHookExW(
                winuser::WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                ptr::null_mut(),
                0,
            );

            assert!(
                !keyboard_hook.is_null(),
                "Failed to install the keyboard hook"
            );

            let mouse_hook = winuser::SetWindowsHookExW(
                winuser::WH_MOUSE_LL,
                Some(low_level_mouse_proc),
                ptr::null_mut(),
                0,
            );

            assert!(!mouse_hook.is_null(), "Failed to install the mouse hook");

            // The message loop has now started.
            // It is ready to receive events.
            STATE.store(2, Ordering::SeqCst);

            let mut message = MaybeUninit::uninit();
            loop {
                let result = winuser::PeekMessageW(
                    message.as_mut_ptr(),
                    ptr::null_mut(),
                    0,
                    0,
                    winuser::PM_REMOVE,
                );

                if result < 0 || STATE.load(Ordering::SeqCst) == 3 {
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            // The message loop is now exiting.

            // Deinitialize the sender
            ptr::drop_in_place(SENDER.as_mut_ptr());

            // Free the installed hooks
            winuser::UnhookWindowsHookEx(keyboard_hook);
            winuser::UnhookWindowsHookEx(mouse_hook);

            // The message loop is now shut down.
            STATE.store(0, Ordering::SeqCst);
        }
    });

    block_until_state_is(2);
    // The message loop successfully started.

    EventReceiver { receiver: r }
}

/// An event of any kind.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    Keyboard {
        /// The virtual keycode of the key that was pressed.
        vk: Vk,
        /// The scan code of that key.
        scan_code: u32,
        /// The action that was taken on the key.
        action: Action,
    },
    MouseMove {
        /// The x coordinate of the mouse, in [per-monitor-aware] screen coordinates.
        ///
        /// [per-monitor-aware]: https://docs.microsoft.com/en-us/windows/desktop/api/shellscalingapi/ne-shellscalingapi-process_dpi_awareness
        x: i32,
        /// The y coordinate of the mouse, in [per-monitor-aware] screen coordinates.
        ///
        /// [per-monitor-aware]: https://docs.microsoft.com/en-us/windows/desktop/api/shellscalingapi/ne-shellscalingapi-process_dpi_awareness
        y: i32,
    },
    MouseButton {
        /// The x coordinate of the mouse, in [per-monitor-aware] screen coordinates.
        ///
        /// [per-monitor-aware]: https://docs.microsoft.com/en-us/windows/desktop/api/shellscalingapi/ne-shellscalingapi-process_dpi_awareness
        x: i32,
        /// The y coordinate of the mouse, in [per-monitor-aware] screen coordinates.
        ///
        /// [per-monitor-aware]: https://docs.microsoft.com/en-us/windows/desktop/api/shellscalingapi/ne-shellscalingapi-process_dpi_awareness
        y: i32,
        /// The action that was taken on the mouse button.
        action: Action,
        /// The mouse button involved in the event.
        button: Button,
    },
    MouseWheel {
        /// The x coordinate of the mouse, in [per-monitor-aware] screen coordinates.
        ///
        /// [per-monitor-aware]: https://docs.microsoft.com/en-us/windows/desktop/api/shellscalingapi/ne-shellscalingapi-process_dpi_awareness
        x: i32,
        /// The y coordinate of the mouse, in [per-monitor-aware] screen coordinates.
        ///
        /// [per-monitor-aware]: https://docs.microsoft.com/en-us/windows/desktop/api/shellscalingapi/ne-shellscalingapi-process_dpi_awareness
        y: i32,
        /// The amount of rotation of the wheel. Positive values indicate that the wheel
        /// was rotated forward, away from the user; a negative value means that the wheel
        /// was rotated backward, toward the user.
        delta: f32,
    },
}

// Only one instance of `EventReceiver` can be created at any given time.
// That only instance relies on `STATE` and `SENDER`.

/// The result of the `start` function. This structure receives the messages
/// received by the message loop.
pub struct EventReceiver {
    receiver: mpsc::Receiver<Event>,
}

impl EventReceiver {
    /// Blocks the current thread until an event is received.
    pub fn next_event(&self) -> Event {
        self.receiver.recv().unwrap()
    }

    /// Blocks the current thread until an event is received or the given
    /// duration is reached.
    pub fn next_event_timeout(&self, timeout: Duration) -> Option<Event> {
        self.receiver.recv_timeout(timeout).ok()
    }

    /// Tries to receive an event without blocking the thread.
    pub fn try_next_event(&self) -> Option<Event> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for EventReceiver {
    fn drop(&mut self) {
        // If the `EventReceiver` was able to be constructed,
        // that means that `STATE` is currently `2`.
        STATE.store(3, Ordering::SeqCst);

        // Cleaning up the static variables is up to the message loop thread.
        block_until_state_is(0);
    }
}
