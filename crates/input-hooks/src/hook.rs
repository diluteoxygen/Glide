use crossbeam_channel::{unbounded, Receiver, Sender};
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_LCONTROL, VK_LSHIFT, VK_RCONTROL, VK_RSHIFT, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, MSG,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_MOUSEWHEEL, WM_SYSKEYDOWN, WM_SYSKEYUP, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT,
};
use crate::events::OtfInputEvent;

const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(350);

lazy_static! {
    static ref EVENT_SENDER: Mutex<Option<Sender<OtfInputEvent>>> = Mutex::new(None);
}

static mut KBD_HOOK: HHOOK = HHOOK(std::ptr::null_mut());
static mut MOUSE_HOOK: HHOOK = HHOOK(std::ptr::null_mut());

// We track physical state to avoid autorepeat false triggers (down -> up -> down).
static SHIFT_IS_DOWN: AtomicBool = AtomicBool::new(false);
static CTRL_IS_DOWN: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref LAST_SHIFT_DOWN: Mutex<Option<Instant>> = Mutex::new(None);
    static ref LAST_CTRL_DOWN: Mutex<Option<Instant>> = Mutex::new(None);
}

fn send_event(event: OtfInputEvent) {
    if let Ok(guard) = EVENT_SENDER.lock() {
        if let Some(sender) = guard.as_ref() {
            let _ = sender.send(event);
        }
    }
}

unsafe extern "system" fn kbd_callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let msg = wparam.0 as u32;
        let kbd = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        
        let vk = kbd.vkCode;
        let is_shift = vk == VK_SHIFT.0 as u32 || vk == VK_LSHIFT.0 as u32 || vk == VK_RSHIFT.0 as u32 || vk == 0xA0 /* VK_LSHIFT */ || vk == 0xA1 /* VK_RSHIFT */;
        let is_ctrl = vk == VK_CONTROL.0 as u32 || vk == VK_LCONTROL.0 as u32 || vk == VK_RCONTROL.0 as u32 || vk == 0xA2 /* VK_LCONTROL */ || vk == 0xA3 /* VK_RCONTROL */;

        if is_shift {
            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                let was_down = SHIFT_IS_DOWN.swap(true, Ordering::SeqCst);
                if !was_down {
                    // Genuine new press (not autorepeat)
                    let now = Instant::now();
                    let mut last = LAST_SHIFT_DOWN.lock().unwrap();
                    if let Some(t) = *last {
                        if now.duration_since(t) <= DOUBLE_TAP_WINDOW {
                            send_event(OtfInputEvent::DoubleTapShift);
                            *last = None; // reset to require two more taps
                        } else {
                            *last = Some(now);
                        }
                    } else {
                        *last = Some(now);
                    }
                }
            } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
                SHIFT_IS_DOWN.store(false, Ordering::SeqCst);
            }
        }
        
        if is_ctrl {
            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                let was_down = CTRL_IS_DOWN.swap(true, Ordering::SeqCst);
                if !was_down {
                    // Genuine new press
                    let now = Instant::now();
                    let mut last = LAST_CTRL_DOWN.lock().unwrap();
                    if let Some(t) = *last {
                        if now.duration_since(t) <= DOUBLE_TAP_WINDOW {
                            send_event(OtfInputEvent::DoubleTapCtrl);
                            *last = None;
                        } else {
                            *last = Some(now);
                        }
                    } else {
                        *last = Some(now);
                    }
                }
            } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
                CTRL_IS_DOWN.store(false, Ordering::SeqCst);
            }
        }
    }
    unsafe { CallNextHookEx(KBD_HOOK, code, wparam, lparam) }
}

unsafe extern "system" fn mouse_callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let msg = wparam.0 as u32;
        if msg == WM_MOUSEWHEEL {
            let msll = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
            // High word of mouseData contains wheel delta
            let delta = ((msll.mouseData >> 16) & 0xFFFF) as i16 as i32;
            if SHIFT_IS_DOWN.load(Ordering::SeqCst) {
                send_event(OtfInputEvent::ShiftScroll(delta));
            }
        }
    }
    unsafe { CallNextHookEx(MOUSE_HOOK, code, wparam, lparam) }
}

pub struct InputHook;

impl InputHook {
    pub fn start() -> Receiver<OtfInputEvent> {
        let (tx, rx) = unbounded();
        *EVENT_SENDER.lock().unwrap() = Some(tx.clone());

        thread::spawn(|| {
            unsafe {
                KBD_HOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(kbd_callback), None, 0).unwrap();
                MOUSE_HOOK = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_callback), None, 0).unwrap();
                
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).into() {
                    // The loop keeps hooks alive
                }
                
                UnhookWindowsHookEx(KBD_HOOK).unwrap();
                UnhookWindowsHookEx(MOUSE_HOOK).unwrap();
            }
        });
        
        crate::cursor::start_cursor_polling(tx);
        
        rx
    }
}
