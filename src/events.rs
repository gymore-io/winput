use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::RwLock;

use winapi::um::winuser;

use crate::input::{Action, Button};
use crate::vk::Vk;

/// The current state of the message loop.
///
/// 0 -> shutdown | the message loop is not active.
/// 1 -> starting | the message loop is starting.
/// 2 -> active   | the message loop is active.
/// 3 -> stopping | the message loop is stopping.
/// 4 -> free     | the dispatcher loop has stopped.
static STATE: AtomicU8 = AtomicU8::new(0);

// Those values are always initialized if `STARTED` is `true`.
// `SENDER` must only be used on the message loop's thread.
static mut SENDER: MaybeUninit<mpsc::Sender<Event>> = MaybeUninit::uninit();
static mut HANDLERS: MaybeUninit<RwLock<HashMap<usize, Box<dyn RawHandler>>>> =
    MaybeUninit::uninit();

static NEXT_HANDLER_ID: AtomicUsize = AtomicUsize::new(0);

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

/// Starts the global message loop and the global dispatch loop on two separate
/// threads.
/// This function will block until both loops started.
///
/// ## Panics
///
/// This function panics if the message loop was already started.
fn start_message_loop() {
    assert_eq!(
        STATE.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst),
        Ok(0),
        "The global message loop was already active"
    );

    // The message loop is now starting.

    // `started` is now `true`, we have to initialize `SENDER`.
    let (s, r) = mpsc::channel();
    unsafe { SENDER = MaybeUninit::new(s) };
    unsafe { HANDLERS = MaybeUninit::new(RwLock::new(HashMap::new())) };

    std::thread::spawn(move || {
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

                if result < 0 || STATE.load(Ordering::SeqCst) == 4 {
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            // The message loop is now stopping.

            // Deinitialize the sender
            ptr::drop_in_place(SENDER.as_mut_ptr());

            // Free the installed hooks
            winuser::UnhookWindowsHookEx(keyboard_hook);
            winuser::UnhookWindowsHookEx(mouse_hook);

            // The message loop is now shut down.
            STATE.store(0, Ordering::SeqCst)
        }
    });

    // We need to use a second thread because any work done on the actual message
    // loop will block Windows' message system.

    std::thread::spawn(move || {
        // Block until the message loop started.
        block_until_state_is(2);

        loop {
            if let Ok(event) = r.try_recv() {
                // SAFETY: HANDLER is known to be initialized at the point.
                let lock = unsafe { &*HANDLERS.as_ptr() }.read().unwrap();

                for (_, handler) in lock.iter() {
                    handler.handle_event(event);
                }
            }

            if STATE.load(Ordering::SeqCst) == 3 {
                // The message loop is stopping
                STATE.store(4, Ordering::SeqCst);
                break;
            }
        }
    });

    block_until_state_is(2);

    // The message loop successfully started.
}

/// A handle to a `Handler`. This handle is returned by the [`subscribe_handler`] function
/// and can be used to unsubscribe it using [`unsubscribe_handler`].
///
/// [`subscribe_handler`]: fn.subscribe_handler.html
/// [`unsubscribe_handler`]: struct.HandlerHandle.html
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandlerHandle(usize);

/// Subscribes a new handler to the message loop. If the message loop was not already active,
/// this function starts it on a separate thread.
///
/// ## Example
///
/// ```rust, ignore
/// use winput::events::{self, Handler};
///
/// struct MyHandler;
/// impl Handler for MyHandle {}
///
/// let h = events::subscribe_handler(MyHandler);
/// std::thread::sleep(std::time::Duration::from_secs(6))
/// events::unsubscribe_handle(h);
/// ```
pub fn subscribe_handler<H: 'static>(handler: H) -> HandlerHandle
where
    H: RawHandler,
{
    if STATE.load(Ordering::SeqCst) == 0 {
        start_message_loop();
    }

    let id = NEXT_HANDLER_ID.fetch_add(1, Ordering::Relaxed);

    // SAFETY: When the `start_message_loop` returns, the message loop is started
    // thus the `HANDLERS` static variable is initialized.
    unsafe {
        (&*HANDLERS.as_ptr())
            .write()
            .unwrap()
            .insert(id, Box::new(handler));
    }

    HandlerHandle(id)
}

// Unsubscribes a handler. If no handlers are left, the message loop is stopped.
///
/// For more information, see [`subscribe_handler`].
///
/// [`subscribe_handler`]: fn.subscribe_handler.html
pub fn unsubscribe_handler(handle: HandlerHandle) {
    if STATE.load(Ordering::SeqCst) == 0 {
        return;
    }

    // SAFETY:
    // `STATE != 0` => `HANDLERS` is initialized
    let mut handlers = unsafe { (&*HANDLERS.as_ptr()).write().unwrap() };

    handlers.remove(&handle.0);

    if handlers.is_empty() {
        // There is no handler left.
        // We can stop the message loop.
        stop();
    }
}

/// Stops the event loop.
fn stop() {
    // The message loop is not active
    if STATE.load(Ordering::SeqCst) == 0 {
        return;
    }

    // The message loop was starting or is currently active.
    block_until_state_is(2);

    // The message loop is now active.

    // Let's ask it to shutdown.
    STATE.store(3, Ordering::SeqCst);

    // Wait until the message loop is actually stoped.
    block_until_state_is(0);
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

/// A raw handler that receive every events at the same place.
pub trait RawHandler: Sync + Send {
    /// Handles the given event.
    fn handle_event(&self, event: Event);
}

pub trait Handler {
    /// A keyboard key was pressed or released.
    fn keyboard(&self, _vk: Vk, _scan_code: u32, _action: Action) {}

    /// The mouse moved.
    fn mouse_move(&self, _x: i32, _y: i32) {}

    /// A mouse button was pressed or released.
    fn mouse_button(&self, _x: i32, _y: i32, _button: Button, _action: Action) {}

    /// The mouse wheel was rotated.
    fn mouse_wheel(&self, _x: i32, _y: i32, _delta: f32) {}
}

impl<H: Handler + Send + Sync> RawHandler for H {
    #[inline]
    fn handle_event(&self, event: Event) {
        match event {
            Event::Keyboard {
                vk,
                scan_code,
                action,
            } => self.keyboard(vk, scan_code, action),
            Event::MouseMove { x, y } => self.mouse_move(x, y),
            Event::MouseButton {
                x,
                y,
                button,
                action,
            } => self.mouse_button(x, y, button, action),
            Event::MouseWheel { x, y, delta } => self.mouse_wheel(x, y, delta),
        }
    }
}
