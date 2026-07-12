use crossbeam_channel::Sender;
use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    Arc,
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Bgra,
    Nv12,
}

pub struct Frame {
    pub data: Vec<u8>,
    pub format: PixelFormat,
    pub width: u32,
    pub height: u32,
    pub timestamp_us: u64,
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("Failed to initialize capture: {0}")]
    Initialization(String),
    #[error("Capture stream error: {0}")]
    StreamError(String),
    #[error("Timeout acquiring frame")]
    Timeout,
}

pub trait VideoCapturer {
    /// Starts the capture loop on the current thread. Blocks until `stop` is true.
    /// Pushes captured `Frame`s to `tx`. Drops frames if `tx` is full.
    ///
    /// Increments `dropped_frames` counter if `tx.try_send` fails.
    fn start(
        &mut self,
        tx: Sender<Frame>,
        stop: Arc<AtomicBool>,
        dropped_frames: Arc<AtomicU64>,
        start_time: Arc<AtomicU64>,
    ) -> Result<(), CaptureError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioTrack {
    SystemLoopback,
    Microphone,
}

pub struct AudioFrame {
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub track: AudioTrack,
    pub timestamp_us: u64,
}

pub trait AudioCapturer {
    /// Starts the audio capture loop on the current thread. Blocks until `stop` is true.
    /// Pushes captured `AudioFrame`s to `tx`.
    fn start(
        &mut self,
        tx: Sender<AudioFrame>,
        stop: Arc<AtomicBool>,
        start_time: Arc<AtomicU64>,
    ) -> Result<(), CaptureError>;
}
