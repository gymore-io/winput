//! The `message_loop` module provides a way to retreive keyboard and mouse
//! input messages directly from the system.
//!
//! Internally, a [message-only window](https://docs.microsoft.com/en-us/windows/win32/winmsg/window-features#message-only-windows)
//! is created to receive the messages.
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

use std::ffi::OsStr;
use std::mem::MaybeUninit;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc;
use std::time::Duration;
use std::{iter, mem, ptr};

use winapi::shared::{hidusage, minwindef, windef};
use winapi::um::{libloaderapi, winuser};

use crate::input::{Action, Button};
use crate::vk::Vk;
use crate::{WheelDirection, WindowsError};

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

/// A buffer that must only be used on the message loop's thread. This buffer must
/// be properly initialized when the message loop's thread is started.
static mut BUFFER: MaybeUninit<Vec<u8>> = MaybeUninit::uninit();

/// Checks whether `short` contains all the bits of `mask`.
#[inline]
fn has_flags(short: u16, mask: u16) -> bool {
    short & mask == mask
}

/// A callback function called by the system on the message loop thread.
unsafe extern "system" fn window_proc(
    hwnd: windef::HWND,
    msg: minwindef::UINT,
    w_param: minwindef::WPARAM,
    l_param: minwindef::LPARAM,
) -> minwindef::LRESULT {
    match msg {
        // Note: This loop is only here to break from the scope early.
        winuser::WM_INPUT => loop {
            // Determine how big should our buffer be.
            let mut size = 0;
            let mut result = winuser::GetRawInputData(
                l_param as winuser::HRAWINPUT,
                winuser::RID_INPUT,
                ptr::null_mut(),
                &mut size,
                mem::size_of::<winuser::RAWINPUTHEADER>() as _,
            );

            if result == -1i32 as u32 {
                break;
            }

            // SAFETY:
            // The buffer must be initialized because we are on the message loop's
            // thread.
            let buffer = &mut *BUFFER.as_mut_ptr();
            buffer.clear();
            buffer.reserve(size as _);

            // Actually write to the buffer.
            result = winuser::GetRawInputData(
                l_param as winuser::HRAWINPUT,
                winuser::RID_INPUT,
                buffer.as_mut_ptr() as _,
                &mut size,
                mem::size_of::<winuser::RAWINPUTHEADER>() as _,
            );

            if result != size {
                // We failed to write to the buffer.
                break;
            }

            // SAFETY:
            // The `GetRawInputData` function did not failed.
            let raw_input = &*(buffer.as_mut_ptr() as winuser::PRAWINPUT);

            // SAFETY:
            // We are on the message loop's thread, `SENDER` must be initialized.
            let sender = &mut *SENDER.as_mut_ptr();

            match raw_input.header.dwType {
                winuser::RIM_TYPEMOUSE => {
                    // Mouse event
                    let data = raw_input.data.mouse();

                    if has_flags(data.usFlags, winuser::MOUSE_MOVE_RELATIVE) {
                        sender
                            .send(Event::MouseMoveRelative {
                                x: data.lLastX,
                                y: data.lLastY,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usFlags, winuser::MOUSE_MOVE_ABSOLUTE) {
                        sender
                            .send(Event::MouseMoveAbsolute {
                                x: data.lLastX as f32 / 65535.0,
                                y: data.lLastY as f32 / 65535.0,
                                virtual_desk: data.usFlags
                                    & winuser::MOUSE_VIRTUAL_DESKTOP
                                    == winuser::MOUSE_VIRTUAL_DESKTOP,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_LEFT_BUTTON_DOWN) {
                        sender
                            .send(Event::MouseButton {
                                action: Action::Press,
                                button: Button::Left,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_LEFT_BUTTON_UP) {
                        sender
                            .send(Event::MouseButton {
                                action: Action::Release,
                                button: Button::Left,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_RIGHT_BUTTON_DOWN)
                    {
                        sender
                            .send(Event::MouseButton {
                                action: Action::Press,
                                button: Button::Right,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_RIGHT_BUTTON_UP) {
                        sender
                            .send(Event::MouseButton {
                                action: Action::Release,
                                button: Button::Right,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_MIDDLE_BUTTON_DOWN)
                    {
                        sender
                            .send(Event::MouseButton {
                                action: Action::Press,
                                button: Button::Middle,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_MIDDLE_BUTTON_UP) {
                        sender
                            .send(Event::MouseButton {
                                action: Action::Release,
                                button: Button::Middle,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_BUTTON_4_DOWN) {
                        sender
                            .send(Event::MouseButton {
                                action: Action::Press,
                                button: Button::X1,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_BUTTON_4_UP) {
                        sender
                            .send(Event::MouseButton {
                                action: Action::Release,
                                button: Button::X1,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_BUTTON_5_DOWN) {
                        sender
                            .send(Event::MouseButton {
                                action: Action::Press,
                                button: Button::X2,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_BUTTON_5_UP) {
                        sender
                            .send(Event::MouseButton {
                                action: Action::Release,
                                button: Button::X2,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, winuser::RI_MOUSE_WHEEL) {
                        sender
                            .send(Event::MouseWheel {
                                delta: data.usButtonData as i16 as f32 / 120.0,
                                direction: WheelDirection::Vertical,
                            })
                            .unwrap();
                    }

                    if has_flags(data.usButtonFlags, 0x0800) {
                        sender
                            .send(Event::MouseWheel {
                                delta: data.usButtonData as i16 as f32 / 120.0,
                                direction: WheelDirection::Horizontal,
                            })
                            .unwrap();
                    }
                }
                winuser::RIM_TYPEKEYBOARD => {
                    // Keyboard event
                    let data = raw_input.data.keyboard();

                    sender
                        .send(Event::Keyboard {
                            vk: Vk::from_u8(data.VKey as u8),
                            scan_code: data.MakeCode as u32,
                            action: Action::from_press(data.Flags & 1 == 0),
                        })
                        .unwrap();
                }
                2 => (),
                _ => unreachable!("Invalid message"),
            }

            break;
        },

        _ => (),
    }

    winuser::DefWindowProcW(hwnd, msg, w_param, l_param)
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

    /// Windows raised an error.
    OsError(WindowsError),
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
/// let receiver = message_loop::start().unwrap();
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

    // We have to initialize `SENDER` and `BUFFER`.
    unsafe {
        SENDER = MaybeUninit::new(s);
        BUFFER = MaybeUninit::new(Vec::new());
    }

    // This channel is used to retreive a potential error from the message loop's
    // thread.
    let (error_s, error_r) = mpsc::channel();

    let thread = stoppable_thread::spawn(move |shouldstop| {
        unsafe {
            // Retreives the module handle of the application.
            let h_instance = libloaderapi::GetModuleHandleW(ptr::null());

            // Create the window.
            let class_name = OsStr::new("winput_message_loop")
                .encode_wide()
                .chain(iter::once(0))
                .collect::<Vec<_>>();

            let mut wnd_class: winuser::WNDCLASSW = mem::zeroed();
            wnd_class.hInstance = h_instance;
            wnd_class.lpszClassName = class_name.as_ptr();
            wnd_class.lpfnWndProc = Some(window_proc);

            let class = winuser::RegisterClassW(&wnd_class);

            if class == 0 {
                error_s
                    .send(Err(MessageLoopError::OsError(
                        WindowsError::from_last_error(),
                    )))
                    .unwrap();
                return;
            }

            let h_wnd = winuser::CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                winuser::HWND_MESSAGE,
                ptr::null_mut(),
                h_instance,
                ptr::null_mut(),
            );

            if h_wnd.is_null() {
                error_s
                    .send(Err(MessageLoopError::OsError(
                        WindowsError::from_last_error(),
                    )))
                    .unwrap();
                return;
            }

            // Tell the system we want to receive inputs.
            let mut rid: [winuser::RAWINPUTDEVICE; 2] = mem::zeroed();
            // Keyboard
            rid[0].dwFlags = winuser::RIDEV_NOLEGACY | winuser::RIDEV_INPUTSINK;
            rid[0].usUsagePage = hidusage::HID_USAGE_PAGE_GENERIC;
            rid[0].usUsage = hidusage::HID_USAGE_GENERIC_KEYBOARD;
            rid[0].hwndTarget = h_wnd;
            // Mouse
            rid[1].dwFlags = winuser::RIDEV_NOLEGACY | winuser::RIDEV_INPUTSINK;
            rid[1].usUsagePage = hidusage::HID_USAGE_PAGE_GENERIC;
            rid[1].usUsage = hidusage::HID_USAGE_GENERIC_MOUSE;
            rid[1].hwndTarget = h_wnd;

            let result = winuser::RegisterRawInputDevices(
                rid.as_ptr(),
                rid.len() as _,
                mem::size_of::<winuser::RAWINPUTDEVICE>() as _,
            );

            if result == 0 {
                error_s
                    .send(Err(MessageLoopError::OsError(
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

            // Start the message loop.
            let mut msg = mem::zeroed();
            while !shouldstop.get() {
                let result = winuser::GetMessageW(&mut msg, h_wnd, 0, 0);

                if result == -1 {
                    // An error occured in the message loop.
                    break;
                } else {
                    winuser::TranslateMessage(&msg);
                    winuser::DispatchMessageW(&msg);
                }
            }

            // The message loop is now exiting.

            // Deinitialize the sender and the buffer.
            // TODO: Use `MaybeUninit::assume_init_drop` when stable.
            ptr::drop_in_place(SENDER.as_mut_ptr());
            ptr::drop_in_place(BUFFER.as_mut_ptr());

            // The message loop is now shut down.
            STATE.store(0, Ordering::SeqCst);
        }
    });

    error_r
        .recv()
        .unwrap()
        .map(|()| EventReceiver { receiver: r, thread: Some(thread) })
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
    MouseMoveRelative {
        /// The x coordinate of the mouse, in [per-monitor-aware] screen coordinates.
        ///
        /// [per-monitor-aware]: https://docs.microsoft.com/en-us/windows/desktop/api/shellscalingapi/ne-shellscalingapi-process_dpi_awareness
        x: i32,
        /// The y coordinate of the mouse, in [per-monitor-aware] screen coordinates.
        ///
        /// [per-monitor-aware]: https://docs.microsoft.com/en-us/windows/desktop/api/shellscalingapi/ne-shellscalingapi-process_dpi_awareness
        y: i32,
    },
    MouseMoveAbsolute {
        /// The x coordinate of the mouse in screen coordinates.
        x: f32,
        /// The y coordinate of the mouse in screen coordinates.
        y: f32,
        /// If this flag is set to `true`, the `x` and `y` coordinates map to the entier
        /// virtual desktop (this is relevent if multiple monitors are used).
        virtual_desk: bool,
    },
    MouseButton {
        /// The action that was taken on the mouse button.
        action: Action,
        /// The mouse button involved in the event.
        button: Button,
    },
    MouseWheel {
        /// The amount of rotation of the wheel. Positive values indicate that the wheel
        /// was rotated forward, away from the user; a negative value means that the wheel
        /// was rotated backward, toward the user.
        delta: f32,
        /// The direction of the wheel.
        direction: WheelDirection,
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
    thread: Option<stoppable_thread::StoppableHandle<()>>
}

impl EventReceiver {
    /// Discard all the events stored in the receiver.
    #[inline]
    pub fn clear(&self) {
        if is_active() {
            while let Some(_) = self.try_next_event() {}
        }
    }
    
    /// Stop the thread inside of the current reciever
    pub fn stop(&mut self) {
        self.thread.take().unwrap().stop().join();
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
