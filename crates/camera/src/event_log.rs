use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::path::PathBuf;
use std::thread::JoinHandle;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(tag = "type")]
pub enum MouseEvent {
    #[serde(rename = "move")]
    Move { x: i32, y: i32 },
    #[serde(rename = "click")]
    Click {
        x: i32,
        y: i32,
        button: String,
        action: String,
    },
    #[serde(rename = "scroll")]
    Scroll { x: i32, y: i32, delta: i32 },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct LogEntry {
    pub t: u64, // Normalized microseconds
    #[serde(flatten)]
    pub event: MouseEvent,
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::Duration;
    use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetCursorPos, GetMessageW, PostThreadMessageW,
        SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HHOOK, MSLLHOOKSTRUCT, MSG,
        WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
        WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP,
    };

    struct SendHook(HHOOK);
    unsafe impl Send for SendHook {}
    unsafe impl Sync for SendHook {}

    static EVENT_SENDER: parking_lot::Mutex<Option<crossbeam_channel::Sender<LogEntry>>> =
        parking_lot::Mutex::new(None);

    static HOOK_HANDLE: parking_lot::Mutex<Option<SendHook>> = parking_lot::Mutex::new(None);

    fn get_qpc_us() -> u64 {
        let mut count = 0;
        let mut freq = 0;
        unsafe {
            let _ = QueryPerformanceCounter(&mut count);
            let _ = QueryPerformanceFrequency(&mut freq);
        }
        if freq > 0 {
            (count as u64 * 1_000_000) / freq as u64
        } else {
            0
        }
    }

    unsafe extern "system" fn mouse_hook_proc(
        ncode: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if ncode >= 0 {
            let ms_struct = *(lparam.0 as *const MSLLHOOKSTRUCT);
            let x = ms_struct.pt.x;
            let y = ms_struct.pt.y;
            let msg_id = wparam.0 as u32;

            let mut mouse_event = None;

            if msg_id == WM_LBUTTONDOWN {
                mouse_event = Some(MouseEvent::Click {
                    x,
                    y,
                    button: "left".to_string(),
                    action: "down".to_string(),
                });
            } else if msg_id == WM_LBUTTONUP {
                mouse_event = Some(MouseEvent::Click {
                    x,
                    y,
                    button: "left".to_string(),
                    action: "up".to_string(),
                });
            } else if msg_id == WM_RBUTTONDOWN {
                mouse_event = Some(MouseEvent::Click {
                    x,
                    y,
                    button: "right".to_string(),
                    action: "down".to_string(),
                });
            } else if msg_id == WM_RBUTTONUP {
                mouse_event = Some(MouseEvent::Click {
                    x,
                    y,
                    button: "right".to_string(),
                    action: "up".to_string(),
                });
            } else if msg_id == WM_MBUTTONDOWN {
                mouse_event = Some(MouseEvent::Click {
                    x,
                    y,
                    button: "middle".to_string(),
                    action: "down".to_string(),
                });
            } else if msg_id == WM_MBUTTONUP {
                mouse_event = Some(MouseEvent::Click {
                    x,
                    y,
                    button: "middle".to_string(),
                    action: "up".to_string(),
                });
            } else if msg_id == WM_MOUSEWHEEL {
                // High-order word of mouseData contains the wheel delta (usually +-120)
                let delta = (ms_struct.mouseData >> 16) as i16 as i32;
                mouse_event = Some(MouseEvent::Scroll { x, y, delta });
            }

            if let Some(event) = mouse_event {
                let sender_guard = EVENT_SENDER.lock();
                if let Some(ref sender) = *sender_guard {
                    let entry = LogEntry {
                        t: get_qpc_us(), // Raw QPC timestamp, normalized later by receiver
                        event,
                    };
                    let _ = sender.send(entry);
                }
            }
        }

        CallNextHookEx(None, ncode, wparam, lparam)
    }

    pub struct WindowsEventTracker {
        writer_thread: Option<JoinHandle<()>>,
        poll_thread: Option<JoinHandle<()>>,
        hook_thread: Option<JoinHandle<()>>,
        hook_thread_id: Arc<std::sync::atomic::AtomicU32>,
    }

    impl WindowsEventTracker {
        pub fn start(
            output_path: PathBuf,
            stop: Arc<AtomicBool>,
            start_time: Arc<AtomicU64>,
        ) -> Result<Self, String> {
            let (tx, rx) = crossbeam_channel::bounded::<LogEntry>(5000);
            *EVENT_SENDER.lock() = Some(tx);

            let mut file = File::create(&output_path)
                .map_err(|e| format!("Failed to create event log file: {}", e))?;

            // 1. Spawn Writer Thread (monotonically writes entries to file)
            let stop_writer = Arc::clone(&stop);
            let start_time_writer = Arc::clone(&start_time);
            let writer_thread = std::thread::spawn(move || {
                tracing::info!("Event log writer thread started.");
                while !stop_writer.load(Ordering::Relaxed) || !rx.is_empty() {
                    if let Ok(mut entry) = rx.recv_timeout(Duration::from_millis(50)) {
                        // Normalize the timestamp
                        let base_time = start_time_writer.load(Ordering::Relaxed);
                        if base_time != u64::MAX {
                            entry.t = entry.t.saturating_sub(base_time);
                            if let Ok(line) = serde_json::to_string(&entry) {
                                let _ = writeln!(file, "{}", line);
                            }
                        }
                    }
                }
                let _ = file.flush();
                tracing::info!("Event log writer thread exited.");
            });

            // 2. Spawn Poller Thread (samples cursor pos at 100Hz)
            let stop_poller = Arc::clone(&stop);
            let tx_poller = EVENT_SENDER.lock().as_ref().unwrap().clone();
            let poll_thread = std::thread::spawn(move || {
                tracing::info!("Cursor position sampler thread started (100Hz).");
                let mut last_pos = None;
                while !stop_poller.load(Ordering::Relaxed) {
                    let mut point = POINT::default();
                    unsafe {
                        if GetCursorPos(&mut point).is_ok() {
                            let new_pos = (point.x, point.y);
                            // Only log if the cursor actually moved
                            if last_pos != Some(new_pos) {
                                let entry = LogEntry {
                                    t: get_qpc_us(),
                                    event: MouseEvent::Move { x: point.x, y: point.y },
                                };
                                let _ = tx_poller.send(entry);
                                last_pos = Some(new_pos);
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                tracing::info!("Cursor position sampler thread exited.");
            });

            // 3. Spawn low-level Mouse Hook Thread
            let hook_thread_id = Arc::new(std::sync::atomic::AtomicU32::new(0));
            let hook_thread_id_clone = Arc::clone(&hook_thread_id);
            let hook_thread = std::thread::spawn(move || {
                tracing::info!("Global low-level mouse hook thread starting.");
                let current_id = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
                hook_thread_id_clone.store(current_id, Ordering::Relaxed);

                unsafe {
                    let hook = SetWindowsHookExW(
                        WH_MOUSE_LL,
                        Some(mouse_hook_proc),
                        None,
                        0,
                    );

                    match hook {
                        Ok(h) => {
                            tracing::info!("Successfully installed low-level mouse hook.");
                            *HOOK_HANDLE.lock() = Some(SendHook(h));

                            // Run Windows message pump (required for hook processing)
                            let mut msg = MSG::default();
                            while GetMessageW(&mut msg, None, 0, 0).into() {
                                let _ = TranslateMessage(&msg);
                                DispatchMessageW(&msg);
                            }

                            // Uninstall hook before exit
                            if let Some(h) = HOOK_HANDLE.lock().take() {
                                let _ = UnhookWindowsHookEx(h.0);
                                tracing::info!("Unhooked low-level mouse hook.");
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to install mouse hook (falling back to polling-only): {}", e);
                        }
                    }
                }
                tracing::info!("Mouse hook thread exited.");
            });

            Ok(Self {
                writer_thread: Some(writer_thread),
                poll_thread: Some(poll_thread),
                hook_thread: Some(hook_thread),
                hook_thread_id,
            })
        }

        pub fn stop(&mut self) {
            // Unregister global event sender first
            *EVENT_SENDER.lock() = None;

            // Signal hook message pump to quit
            let tid = self.hook_thread_id.load(Ordering::Relaxed);
            if tid != 0 {
                unsafe {
                    let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
                }
            }

            // Join threads
            if let Some(t) = self.poll_thread.take() {
                let _ = t.join();
            }
            if let Some(t) = self.hook_thread.take() {
                let _ = t.join();
            }
            if let Some(t) = self.writer_thread.take() {
                let _ = t.join();
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_impl::WindowsEventTracker as PlatformEventTracker;

#[cfg(not(target_os = "windows"))]
mod stub_impl {
    use super::*;
    pub struct StubEventTracker;
    impl StubEventTracker {
        pub fn start(
            _output_path: PathBuf,
            _stop: Arc<AtomicBool>,
            _start_time: Arc<AtomicU64>,
        ) -> Result<Self, String> {
            tracing::info!("Event logging is not supported on this platform.");
            Ok(Self)
        }
        pub fn stop(&mut self) {}
    }
}

#[cfg(not(target_os = "windows"))]
pub use stub_impl::StubEventTracker as PlatformEventTracker;

pub struct EventTracker {
    inner: PlatformEventTracker,
}

impl EventTracker {
    pub fn start(
        output_path: PathBuf,
        stop: Arc<AtomicBool>,
        start_time: Arc<AtomicU64>,
    ) -> Result<Self, String> {
        let inner = PlatformEventTracker::start(output_path, stop, start_time)?;
        Ok(Self { inner })
    }

    pub fn stop(&mut self) {
        self.inner.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let entry = LogEntry {
            t: 12345,
            event: MouseEvent::Move { x: 100, y: 200 },
        };
        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(serialized.contains(r#""type":"move""#));
        assert!(serialized.contains(r#""t":12345"#));
        assert!(serialized.contains(r#""x":100"#));
        assert!(serialized.contains(r#""y":200"#));

        let entry2 = LogEntry {
            t: 67890,
            event: MouseEvent::Click {
                x: 300,
                y: 400,
                button: "left".to_string(),
                action: "down".to_string(),
            },
        };
        let serialized2 = serde_json::to_string(&entry2).unwrap();
        assert!(serialized2.contains(r#""type":"click""#));
        assert!(serialized2.contains(r#""button":"left""#));
        assert!(serialized2.contains(r#""action":"down""#));

        // Test deserialization
        let deserialized: LogEntry = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.t, 12345);
        match deserialized.event {
            MouseEvent::Move { x, y } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            _ => panic!("Expected MouseEvent::Move"),
        }
    }
}
