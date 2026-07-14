use std::thread;
use std::time::Duration;
use input_hooks::InputHook;
use otf_camera::OtfCameraEngine;

fn main() {
    println!("Starting input hooks...");
    let event_rx = InputHook::start();
    
    // Assume 1080p screen for the trace
    let mut engine = OtfCameraEngine::new(1920.0, 1080.0);
    
    println!("Camera engine started. State: {:?}", engine.state);
    println!("Perform gestures to see the trace. Press Ctrl+C to exit.");
    
    // Run at ~60Hz
    loop {
        let (x, y, zoom) = engine.tick(&event_rx);
        
        let state = format!("{:?}", engine.state);
        // We only print when zoom > 1.0 or if we want continuous print, we print every 10 frames
        // Let's print every tick if we are in Zoomed state, or if zoom > 1.01
        
        if zoom > 1.01 || state.contains("Zoomed") {
            // Plot a simple ASCII trace for the zoom value:
            // 1.0 to 4.0 maps to 0 to 60 chars
            let bars = (((zoom - 1.0) / 3.0).clamp(0.0, 1.0) * 60.0) as usize;
            let bar_str = "=".repeat(bars);
            
            println!("[{:15}] X: {:4.0}, Y: {:4.0} | Z: {:.2} |{}", state, x, y, zoom, bar_str);
        }
        
        thread::sleep(Duration::from_millis(16));
    }
}
