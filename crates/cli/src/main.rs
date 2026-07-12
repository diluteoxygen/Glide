use capture_core::{Frame, VideoCapturer};
use crossbeam_channel::bounded;
use std::env;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::System;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[cfg(target_os = "windows")]
use capture_windows::DxgiCapturer;

#[cfg(target_os = "linux")]
use capture_linux::PipeWireCapturer;

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting Glide CLI (Phase 1 MVP Test Harness)");
    info!("Platform: {}", env::consts::OS);

    // Initialize the appropriate capturer
    #[cfg(target_os = "windows")]
    let mut capturer = DxgiCapturer::new().expect("Failed to initialize DXGI capturer");

    #[cfg(target_os = "linux")]
    let mut capturer = PipeWireCapturer::new().expect("Failed to initialize PipeWire capturer");

    let (tx, rx) = bounded::<Frame>(5);
    let stop = Arc::new(AtomicBool::new(false));
    let dropped_frames = Arc::new(AtomicU64::new(0));

    let stop_clone = Arc::clone(&stop);
    let dropped_clone = Arc::clone(&dropped_frames);

    // Spawn capture thread
    let capture_thread = thread::spawn(move || capturer.start(tx, stop_clone, dropped_clone));

    // Initialize sysinfo to monitor CPU usage
    let mut sys = System::new_all();
    let pid = sysinfo::get_current_pid().expect("Failed to get current PID");

    let mut received_frames = 0;
    let run_duration = Duration::from_secs(10);
    let start_time = Instant::now();
    let mut last_log = Instant::now();

    info!("Running capture for 10 seconds...");

    while start_time.elapsed() < run_duration {
        if let Ok(_frame) = rx.try_recv() {
            received_frames += 1;
        }

        // Log stats every second
        if last_log.elapsed() >= Duration::from_secs(1) {
            sys.refresh_processes();
            let cpu_usage = if let Some(process) = sys.process(pid) {
                process.cpu_usage()
            } else {
                0.0
            };

            let dropped = dropped_frames.load(Ordering::Relaxed);
            let total = received_frames + dropped;
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            let fps = total as f64 / elapsed_secs;

            info!(
                "Stats: {:.1} FPS | Received: {} | Dropped: {} | CPU: {:.1}%",
                fps, received_frames, dropped, cpu_usage
            );

            last_log = Instant::now();
        }

        thread::sleep(Duration::from_millis(5));
    }

    info!("Signaling stop to capture thread...");
    stop.store(true, Ordering::Relaxed);

    // Drain channel to unblock capturer if it's waiting on a full channel
    while rx.try_recv().is_ok() {}

    match capture_thread.join() {
        Ok(Ok(_)) => info!("Capture thread exited cleanly."),
        Ok(Err(e)) => info!("Capture thread returned error: {}", e),
        Err(_) => info!("Capture thread panicked."),
    }

    let total_dropped = dropped_frames.load(Ordering::Relaxed);
    let total_fps = (received_frames + total_dropped) as f64 / 10.0;
    info!("--- Final Results ---");
    info!("Total Received: {}", received_frames);
    info!("Total Dropped:  {}", total_dropped);
    info!("Average FPS:    {:.1}", total_fps);
}
