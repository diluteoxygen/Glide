#![cfg(target_os = "linux")]

pub mod audio;
pub use audio::PipeWireAudioCapturer;

use capture_core::{CaptureError, Frame, PixelFormat, VideoCapturer};
use crossbeam_channel::Sender;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;

pub struct PipeWireCapturer {
    // Pipewire/ashpd state will go here in the future
}

impl PipeWireCapturer {
    pub fn new() -> Result<Self, CaptureError> {
        // Placeholder for ashpd portal handshake and pipewire setup
        // In Phase 1 we just stub the loop for now if we can't test on CI easily,
        // but let's implement a dummy loop that just sends fake frames so we can compile it.
        // The actual PipeWire implementation requires a running loop and DBus session.
        Ok(Self {})
    }
}

impl VideoCapturer for PipeWireCapturer {
    fn start(
        &mut self,
        tx: Sender<Frame>,
        stop: Arc<AtomicBool>,
        dropped_frames: Arc<AtomicU64>,
        start_time: Arc<AtomicU64>,
    ) -> Result<(), CaptureError> {
        let start_time = Instant::now();

        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS

            let frame = Frame {
                data: vec![0; 1920 * 1080 * 4],
                format: PixelFormat::Bgra,
                width: 1920,
                height: 1080,
                timestamp_us: start_time.elapsed().as_micros() as u64,
            };

            if tx.try_send(frame).is_err() {
                dropped_frames.fetch_add(1, Ordering::Relaxed);
            }
        }

        Ok(())
    }
}
