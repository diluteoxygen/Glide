use std::thread;
use std::time::Duration;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows::Win32::Foundation::POINT;
use crossbeam_channel::Sender;
use crate::events::OtfInputEvent;

pub fn start_cursor_polling(tx: Sender<OtfInputEvent>) {
    thread::spawn(move || {
        let mut last_x = -1;
        let mut last_y = -1;
        
        loop {
            let mut pt = POINT { x: 0, y: 0 };
            unsafe {
                let _ = GetCursorPos(&mut pt);
            }
            
            if pt.x != last_x || pt.y != last_y {
                let _ = tx.send(OtfInputEvent::CursorMoved(pt.x, pt.y));
                last_x = pt.x;
                last_y = pt.y;
            }
            
            thread::sleep(Duration::from_millis(10)); // 100Hz
        }
    });
}
