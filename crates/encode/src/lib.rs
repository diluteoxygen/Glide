pub mod audio;
pub mod video;

use capture_core::{AudioFrame, Frame};
use crossbeam_channel::{Receiver, Sender};
use ffmpeg_next as ffmpeg;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use thiserror::Error;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error("Initialization failed: {0}")]
    Initialization(String),
    #[error("Encoding error: {0}")]
    Encoding(String),
}

/// A wrapper struct representing an encoded packet ready for muxing.
pub struct EncodedPacket {
    pub stream_index: usize,
    pub packet: ffmpeg::Packet,
}

pub struct CodecParameters {
    pub video: ffmpeg::codec::Parameters,
    pub video_time_base: ffmpeg::Rational,
    pub audio_sys: ffmpeg::codec::Parameters,
    pub audio_sys_time_base: ffmpeg::Rational,
    pub audio_mic: ffmpeg::codec::Parameters,
    pub audio_mic_time_base: ffmpeg::Rational,
}

pub struct Encoder {
    video_rx: Receiver<Frame>,
    sys_rx: Receiver<AudioFrame>,
    mic_rx: Receiver<AudioFrame>,
    packet_tx: Sender<EncodedPacket>,
    params_tx: Sender<CodecParameters>,
}

impl Encoder {
    pub fn new(
        video_rx: Receiver<Frame>,
        sys_rx: Receiver<AudioFrame>,
        mic_rx: Receiver<AudioFrame>,
        packet_tx: Sender<EncodedPacket>,
        params_tx: Sender<CodecParameters>,
    ) -> Result<Self, EncodeError> {
        ffmpeg::init()
            .map_err(|e| EncodeError::Initialization(format!("FFmpeg init failed: {}", e)))?;
        Ok(Self {
            video_rx,
            sys_rx,
            mic_rx,
            packet_tx,
            params_tx,
        })
    }

    pub fn start(self, stop: Arc<AtomicBool>) -> Result<(), EncodeError> {
        info!("Starting Encode thread...");

        // Initialize encoders
        // For video:
        // Wait for first frame to know resolution
        let first_frame = loop {
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            if let Ok(f) = self.video_rx.try_recv() {
                break f;
            }
            thread::sleep(std::time::Duration::from_millis(5));
        };

        let mut video_encoder = video::VideoEncoder::new(first_frame.width, first_frame.height)?;

        let mut sys_encoder = audio::AudioEncoder::new(48000, 2)?;
        let mut mic_encoder = audio::AudioEncoder::new(48000, 2)?;

        // The stream indices we'll assign to these encoders for the muxer
        let video_stream_idx = 0;
        let sys_stream_idx = 1;
        let mic_stream_idx = 2;

        let codec_params = CodecParameters {
            video: (&video_encoder.encoder).into(),
            video_time_base: video_encoder.time_base,
            audio_sys: (&sys_encoder.encoder).into(),
            audio_sys_time_base: sys_encoder.time_base,
            audio_mic: (&mic_encoder.encoder).into(),
            audio_mic_time_base: mic_encoder.time_base,
        };

        if self.params_tx.send(codec_params).is_err() {
            error!("Failed to send codec parameters to muxer");
            return Ok(());
        }

        // Encode the first frame
        if let Err(e) = video_encoder.encode(first_frame, video_stream_idx, &self.packet_tx) {
            error!("Failed to encode first video frame: {}", e);
        }

        while !stop.load(Ordering::Relaxed) {
            let mut idle = true;

            // Video
            if let Ok(frame) = self.video_rx.try_recv() {
                idle = false;
                if let Err(e) = video_encoder.encode(frame, video_stream_idx, &self.packet_tx) {
                    error!("Video encode error: {}", e);
                }
            }

            // System Audio
            if let Ok(frame) = self.sys_rx.try_recv() {
                idle = false;
                if let Err(e) = sys_encoder.encode(frame, sys_stream_idx, &self.packet_tx) {
                    error!("System audio encode error: {}", e);
                }
            }

            // Mic Audio
            if let Ok(frame) = self.mic_rx.try_recv() {
                idle = false;
                if let Err(e) = mic_encoder.encode(frame, mic_stream_idx, &self.packet_tx) {
                    error!("Mic audio encode error: {}", e);
                }
            }

            if idle {
                thread::sleep(std::time::Duration::from_millis(1));
            }
        }

        info!("Encode thread flushing...");
        video_encoder.flush(video_stream_idx, &self.packet_tx).ok();
        sys_encoder.flush(sys_stream_idx, &self.packet_tx).ok();
        mic_encoder.flush(mic_stream_idx, &self.packet_tx).ok();

        info!("Encode thread exited cleanly.");
        Ok(())
    }
}
