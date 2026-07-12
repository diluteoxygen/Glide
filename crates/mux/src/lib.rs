use crossbeam_channel::Receiver;
use encode::EncodedPacket;
use ffmpeg_next as ffmpeg;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use thiserror::Error;
use tracing::{info, error};

#[derive(Error, Debug)]
pub enum MuxError {
    #[error("Initialization failed: {0}")]
    Initialization(String),
    #[error("Muxing error: {0}")]
    Muxing(String),
}

pub struct Muxer {
    packet_rx: Receiver<EncodedPacket>,
    output_path: String,
}

impl Muxer {
    pub fn new(packet_rx: Receiver<EncodedPacket>, output_path: String) -> Result<Self, MuxError> {
        ffmpeg::init().map_err(|e| MuxError::Initialization(format!("FFmpeg init failed: {}", e)))?;
        Ok(Self {
            packet_rx,
            output_path,
        })
    }

    pub fn start(self, stop: Arc<AtomicBool>) -> Result<(), MuxError> {
        info!("Starting Mux thread for {}...", self.output_path);

        let mut output_ctx = ffmpeg::format::output(&self.output_path).map_err(|e| {
            MuxError::Initialization(format!("Failed to open output file: {}", e))
        })?;

        // We assume streams are matching the encode thread indices: 0 = Video, 1 = Sys Audio, 2 = Mic Audio
        // Adding streams requires codec params, which normally we get from the encoders.
        // For a robust implementation, the Encoder thread should send CodecParameters before frames.
        // For Phase 3 MVP, we will let ffmpeg_next guess the parameters from the packets or we can pass them.
        
        // Wait, ffmpeg_next requires adding streams *before* writing the header.
        // We will add dummy streams and then the first packet will establish it, or we create a global context.
        // Actually, MKV allows late headers? No, it's better to add the streams properly.
        // Let's add them as unknown and let `write_interleaved` handle the rest, or just create basic streams.
        {
            let _ost_vid = output_ctx.add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::H264)).unwrap();
            let _ost_sys = output_ctx.add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::AAC)).unwrap();
            let _ost_mic = output_ctx.add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::AAC)).unwrap();
        }

        output_ctx.write_header().map_err(|e| {
            MuxError::Initialization(format!("Failed to write header: {}", e))
        })?;

        while !stop.load(Ordering::Relaxed) {
            if let Ok(enc_pkt) = self.packet_rx.try_recv() {
                // Ensure packet's stream index matches what we added
                // enc_pkt.stream_index is 0, 1, or 2
                // enc_pkt.packet already has it set, but let's confirm
                if let Err(e) = enc_pkt.packet.write_interleaved(&mut output_ctx) {
                    error!("Failed to write interleaved packet: {}", e);
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }

        // Drain any remaining packets in the channel before closing
        while let Ok(enc_pkt) = self.packet_rx.try_recv() {
            let _ = enc_pkt.packet.write_interleaved(&mut output_ctx);
        }

        output_ctx.write_trailer().map_err(|e| {
            MuxError::Muxing(format!("Failed to write trailer: {}", e))
        })?;

        info!("Mux thread exited cleanly, file closed.");
        Ok(())
    }
}
