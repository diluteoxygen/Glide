use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::Receiver;
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::Graphics::Gdi::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::System::LibraryLoader::*,
};

pub struct OtfOverlay;

impl OtfOverlay {
    pub fn start(
        rx: Receiver<(i32, i32, i32, i32, bool)>,
        stop: Arc<AtomicBool>,
    ) {
        std::thread::Builder::new()
            .name("otf-overlay".into())
            .spawn(move || {
                unsafe {
                    Self::run_loop(rx, stop);
                }
            })
            .expect("Failed to spawn otf-overlay thread");
    }

    unsafe fn run_loop(rx: Receiver<(i32, i32, i32, i32, bool)>, stop: Arc<AtomicBool>) {
        let instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("OtfOverlayClass");

        let wc = WNDCLASSW {
            lpfnWndProc: Some(Self::wndproc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0 as _),
            ..Default::default()
        };

        if RegisterClassW(&wc) == 0 {
            tracing::error!("Failed to register OtfOverlay window class");
            return;
        }

        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);

        let hwnd = match CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class_name,
            w!("Glide OTF Overlay"),
            WS_POPUP,
            0,
            0,
            screen_width,
            screen_height,
            None,
            None,
            instance,
            None,
        ) {
            Ok(hwnd) => hwnd,
            Err(e) => {
                tracing::error!("Failed to create OtfOverlay window: {:?}", e);
                return;
            }
        };

        // Set opacity to 50% (128)
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 128, LWA_ALPHA).unwrap();
        
        // Exclude from capture!
        SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE).unwrap();

        // Initially hidden
        let _ = ShowWindow(hwnd, SW_HIDE);

        let mut msg = MSG::default();
        let mut is_showing = false;

        while !stop.load(Ordering::Relaxed) {
            // Process all pending messages
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                if msg.message == WM_QUIT {
                    return;
                }
                TranslateMessage(&msg);
                unsafe { DispatchMessageW(&msg) };
            }

            // Update region based on latest rect
            // Drain receiver to get the most recent message
            let mut latest_rect = None;
            while let Ok(r) = rx.try_recv() {
                latest_rect = Some(r);
            }

            if let Some((left, top, right, bottom, is_zoomed)) = latest_rect {
                if is_zoomed {
                    if !is_showing {
                        let _ = ShowWindow(hwnd, SW_SHOWNA);
                        is_showing = true;
                    }
                    
                    let screen_rgn = CreateRectRgn(0, 0, screen_width, screen_height);
                    let hole_rgn = CreateRectRgn(left, top, right, bottom);
                    
                    let _ = CombineRgn(screen_rgn, screen_rgn, hole_rgn, RGN_DIFF);
                    // System owns the region once passed to SetWindowRgn
                    let _ = SetWindowRgn(hwnd, screen_rgn, true);
                    
                    let _ = DeleteObject(hole_rgn);
                    // Do not delete screen_rgn! SetWindowRgn takes ownership.
                } else {
                    if is_showing {
                        let _ = ShowWindow(hwnd, SW_HIDE);
                        is_showing = false;
                        let _ = SetWindowRgn(hwnd, HRGN::default(), true);
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(16)); // ~60fps
        }
        
        let _ = DestroyWindow(hwnd);
    }

    unsafe extern "system" fn wndproc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}
