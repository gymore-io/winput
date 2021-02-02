//! The `message_loop` module provides a way to retreive keyboard and mouse
//! input messages directly from the system.
//!
//! Internally, the [`SetWindowsHookEx`] function is used along with the
//! [`WH_KEYBOARD_LL`] and [`WH_MOUSE_LL`] events. Everything gets dispatched
//! by calling [`PeekMessageW`].
//!
//! ## Examples
//!
//! ```rust, ignore
//! use winput::{Vk, Action};
//! use winput::message_loop;
//!
//! let receiver = message_loop::start().unwrap();
//!
//! loop {
//!     match receiver.next_event() {
//!         message_loop::Event::Keyboard {
//!             vk,
//!             action: Action::Press,
//!             ..
//!         } => {
//!             if vk == Vk::Escape {
//!                 break;
//!             } else {
//!                 println!("{:?} was pressed!", vk);
//!             }
//!         },
//!         _ => (),
//!     }
//! }
//! ```
//!
//! [`SetWindowsHookEx`]: https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw
//! [`WH_KEYBOARD_LL`]: https://docs.microsoft.com/en-us/windows/win32/winmsg/about-hooks#wh_keyboard_ll
//! [`WH_MOUSE_LL`]: https://docs.microsoft.com/en-us/windows/win32/winmsg/about-hooks#wh_mouse
//! [`PeekMessageW`]: https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-peekmessagew

use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use winapi::um::winuser;

use crate::input::{Action, Button};
use crate::vk::Vk;
use crate::WindowsError;

/// The current state of the message loop.
///
/// * 0 -> The message loop is not active.
/// * 1 -> The `start` function has been called.
///        The message loop is now starting.
/// * 2 -> The message loop has successfully started.
/// * 3 -> The message loop is now exiting.
static STATE: AtomicU8 = AtomicU8::new(0);

// This value initialized if `STATE` is `2`. It is uninitialized if `STATE` is `0`.
// `SENDER` must only be used on the message loop's thread.
static mut SENDER: MaybeUninit<mpsc::Sender<Event>> = MaybeUninit::uninit();

/// Callback called by Windows' message loop on the message loop's thread when a
/// `WM_KEYBOARD_LL` event is received.
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

/// Callback called by Windows' message loop on the message loop's thread when a
/// `WM_MOUSE_LL` event is received.
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
                button: Button::Left,
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

/// An error that can be produced by the [`start`] function.
///
/// [`start`]: fn.start.html
#[derive(Clone, Debug)]
pub enum MessageLoopError {
    /// Only one message loop can be created at any given time. This error
    /// is produced when [`start`] is called even though the message loop
    /// was already active.
    AlreadyActive,

    /// The function failed to install a hook (the keyboard hook or the mouse
    /// hook).
    HookInstallation(WindowsError),
}

/// Checks if the message loop is currently active. When this function returns
/// `true`, calling `start` always produces an error.
///
/// ## Examples
///
/// ```rust, ignore
/// let _ = winput::messgage_loop::start();
/// assert!(winput::message_loop::is_active());
///
/// ```
#[inline]
pub fn is_active() -> bool {
    STATE.load(Ordering::Acquire) != 0
}

/// Starts the message loop on a new thread.
///
/// ## Returns
///
/// This function returns an error if the message loop is already active: only one
/// message loop can be started at any given time. Be carfull if another library is
/// also using the message loop.
///
/// You can check if the message loop is currently active by calling [`is_active`].
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
///
/// [`is_active`]: fn.is_active.html
pub fn start() -> Result<EventReceiver, MessageLoopError> {
    loop {
        match STATE.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(0) => break,

            // If the message loop is shutting down, we can just wait
            // a bit until we can start it again.
            Err(3) => (),
            _ => return Err(MessageLoopError::AlreadyActive),
        }

        std::hint::spin_loop();
    }

    // The message loop is now starting.
    // This channel is used to receive the messages of the message loop.
    let (s, r) = mpsc::channel();

    // We have to initialize `SENDER`.
    unsafe { SENDER = MaybeUninit::new(s) };

    // This channel is used to retreive a potential error from the message loop's
    // thread.
    let (error_s, error_r) = mpsc::channel();

    std::thread::spawn(move || {
        unsafe {
            // Install the hooks

            let keyboard_hook = winuser::SetWindowsHookExW(
                winuser::WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                ptr::null_mut(),
                0,
            );

            if keyboard_hook.is_null() {
                error_s
                    .send(Err(MessageLoopError::HookInstallation(
                        WindowsError::from_last_error(),
                    )))
                    .unwrap();
                return;
            }

            let mouse_hook = winuser::SetWindowsHookExW(
                winuser::WH_MOUSE_LL,
                Some(low_level_mouse_proc),
                ptr::null_mut(),
                0,
            );

            if mouse_hook.is_null() {
                winuser::UnhookWindowsHookEx(keyboard_hook);

                error_s
                    .send(Err(MessageLoopError::HookInstallation(
                        WindowsError::from_last_error(),
                    )))
                    .unwrap();
                return;
            }

            // The message loop has now started.
            // It is ready to receive events.
            STATE.store(2, Ordering::SeqCst);

            // Notify the main thread that the initialisation is a success.
            error_s.send(Ok(())).unwrap();
            // After this point, the `start` function will return and the receiver
            // will be dropped. Using the `error_s` after this will always return an error.
            drop(error_s);

            let mut message = MaybeUninit::uninit();
            loop {
                // Events are sent through the channel during the call to
                // this function.
                let result = winuser::PeekMessageW(
                    message.as_mut_ptr(),
                    ptr::null_mut(),
                    0,
                    0,
                    winuser::PM_REMOVE,
                );

                if result < 0 || STATE.load(Ordering::Acquire) == 3 {
                    break;
                }

                std::hint::spin_loop();
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

    error_r
        .recv()
        .unwrap()
        .map(|()| EventReceiver { receiver: r })
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
// That only instance relies on `STATE` and `SENDER` that is only initialized
// when `STATE` is `2`.
//
/// The result of the [`start`] function. This structure receives the messages
/// received by the message loop.
///
/// The message loop is automatically stopped when this structure is dropped.
///
/// [`start`]: fn.start.html
pub struct EventReceiver {
    receiver: mpsc::Receiver<Event>,
}

impl EventReceiver {
    /// Discard all the events stored in the receiver.
    #[inline]
    pub fn clear(&self) {
        if is_active() {
            while let Some(_) = self.try_next_event() {}
        }
    }

    /// Blocks the current thread until an event is received.
    #[inline]
    pub fn next_event(&self) -> Event {
        self.receiver
            .recv()
            .expect("The message loop is not active")
    }

    /// Blocks the current thread until an event is received or the given
    /// duration is reached.
    #[inline]
    pub fn next_event_timeout(&self, timeout: Duration) -> Option<Event> {
        match self.receiver.recv_timeout(timeout) {
            Ok(val) => Some(val),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("The message loop is not active")
            }
        }
    }

    /// Tries to receive an event without blocking the thread.
    #[inline]
    pub fn try_next_event(&self) -> Option<Event> {
        match self.receiver.try_recv() {
            Ok(val) => Some(val),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                panic!("The message loop is not active")
            }
        }
    }

    // TODO: add `next_event_deadline` when `Reciever::recv_deadline` is stable.
}

impl Drop for EventReceiver {
    fn drop(&mut self) {
        // Stop the message loop.
        stop();
    }
}

/// Stops the message loop.
///
/// After calling this function, using the `EventReceiver` will always result
/// in a panic.
///
/// Be careful, if another libary already created a message loop, this function will
/// still stop it.
pub fn stop() {
    if !is_active() {
        return;
    }

    // If the `EventReceiver` was able to be constructed,
    // that means that `STATE` is currently `2`.
    STATE.store(3, Ordering::SeqCst);

    // Cleaning up the static variables is up to the message loop thread.
    // We just have to wait until it finishes.
    while STATE.load(Ordering::Acquire) != 0 {
        std::hint::spin_loop();
    }
}
