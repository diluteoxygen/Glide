use crossbeam_channel::Receiver;
use encode::{CodecParameters, EncodedPacket};
use ffmpeg_next as ffmpeg;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use thiserror::Error;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum MuxError {
    #[error("Initialization failed: {0}")]
    Initialization(String),
    #[error("Muxing error: {0}")]
    Muxing(String),
}

pub struct Muxer {
    packet_rx: Receiver<EncodedPacket>,
    params_rx: Receiver<CodecParameters>,
    output_path: String,
}

impl Muxer {
    pub fn new(
        packet_rx: Receiver<EncodedPacket>,
        params_rx: Receiver<CodecParameters>,
        output_path: String,
    ) -> Result<Self, MuxError> {
        ffmpeg::init()
            .map_err(|e| MuxError::Initialization(format!("FFmpeg init failed: {}", e)))?;
        Ok(Self {
            packet_rx,
            params_rx,
            output_path,
        })
    }

    pub fn start(self, stop: Arc<AtomicBool>) -> Result<(), MuxError> {
        info!("Starting Mux thread for {}...", self.output_path);

        let mut output_ctx = ffmpeg::format::output(&self.output_path)
            .map_err(|e| MuxError::Initialization(format!("Failed to open output file: {}", e)))?;

        let codec_params = self.params_rx.recv().map_err(|_| {
            MuxError::Initialization("Encoder channel closed before sending codec parameters".into())
        })?;

        {
            let mut ost_vid = output_ctx
                .add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::H264))
                .map_err(|e| MuxError::Initialization(format!("Failed to add video stream: {}", e)))?;
            ost_vid.set_parameters(codec_params.video);
            ost_vid.set_time_base(codec_params.video_time_base);

            let mut ost_sys = output_ctx
                .add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::AAC))
                .map_err(|e| MuxError::Initialization(format!("Failed to add sys audio stream: {}", e)))?;
            ost_sys.set_parameters(codec_params.audio_sys);
            ost_sys.set_time_base(codec_params.audio_sys_time_base);

            let mut ost_mic = output_ctx
                .add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::AAC))
                .map_err(|e| MuxError::Initialization(format!("Failed to add mic audio stream: {}", e)))?;
            ost_mic.set_parameters(codec_params.audio_mic);
            ost_mic.set_time_base(codec_params.audio_mic_time_base);
        }

        output_ctx
            .write_header()
            .map_err(|e| MuxError::Initialization(format!("Failed to write header: {}", e)))?;

        while !stop.load(Ordering::Relaxed) {
            if let Ok(mut enc_pkt) = self.packet_rx.try_recv() {
                let from_time_base = match enc_pkt.stream_index {
                    0 => codec_params.video_time_base,
                    1 => codec_params.audio_sys_time_base,
                    2 => codec_params.audio_mic_time_base,
                    _ => continue,
                };
                if let Some(stream) = output_ctx.stream(enc_pkt.stream_index) {
                    let to_time_base = stream.time_base();
                    enc_pkt.packet.rescale_ts(from_time_base, to_time_base);
                }
                
                if let Err(e) = enc_pkt.packet.write_interleaved(&mut output_ctx) {
                    error!("Failed to write interleaved packet: {}", e);
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }

        // Drain any remaining packets in the channel before closing
        while let Ok(mut enc_pkt) = self.packet_rx.try_recv() {
            let from_time_base = match enc_pkt.stream_index {
                0 => codec_params.video_time_base,
                1 => codec_params.audio_sys_time_base,
                2 => codec_params.audio_mic_time_base,
                _ => continue,
            };
            if let Some(stream) = output_ctx.stream(enc_pkt.stream_index) {
                let to_time_base = stream.time_base();
                enc_pkt.packet.rescale_ts(from_time_base, to_time_base);
            }
            let _ = enc_pkt.packet.write_interleaved(&mut output_ctx);
        }

        output_ctx
            .write_trailer()
            .map_err(|e| MuxError::Muxing(format!("Failed to write trailer: {}", e)))?;

        info!("Mux thread exited cleanly, file closed.");
        Ok(())
    }
}
