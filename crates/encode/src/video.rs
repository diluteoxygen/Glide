use capture_core::Frame;
use crossbeam_channel::Sender;
use ffmpeg_next as ffmpeg;
use std::collections::HashMap;
use tracing::{info, warn};
use crate::{EncodeError, EncodedPacket};

pub struct VideoEncoder {
    encoder: ffmpeg::codec::encoder::video::Encoder,
    scaler: ffmpeg::software::scaling::Context,
    frame_index: i64,
    width: u32,
    height: u32,
}

impl VideoEncoder {
    pub fn new(width: u32, height: u32) -> Result<Self, EncodeError> {
        unimplemented!("Phase 3: Context::new_with_codec API removal fix")
    }

    pub fn encode(
        &mut self,
        frame: Frame,
        stream_index: usize,
        tx: &Sender<EncodedPacket>,
    ) -> Result<(), EncodeError> {
        // Create an AVFrame for the raw BGRA data
        let mut raw_avframe = ffmpeg::frame::Video::empty();
        let frame_format = ffmpeg::format::Pixel::BGRA;
        
        // Unfortunately, ffmpeg-next Frame::new doesn't easily accept raw bytes for BGRA.
        // We create a new one, then manually fill data.
        let mut raw_avframe = ffmpeg::frame::Video::new(frame_format, self.width, self.height);
        let stride = raw_avframe.stride(0);
        let data = raw_avframe.data_mut(0);

        // Copy raw frame data accounting for stride
        let bytes_per_pixel = 4;
        let line_size = (self.width as usize) * bytes_per_pixel;
        for y in 0..(self.height as usize) {
            let src_start = y * line_size;
            let src_end = src_start + line_size;
            let dst_start = y * stride;
            let dst_end = dst_start + line_size;
            data[dst_start..dst_end].copy_from_slice(&frame.data[src_start..src_end]);
        }

        let mut nv12_avframe = ffmpeg::frame::Video::empty();
        self.scaler.run(&raw_avframe, &mut nv12_avframe).map_err(|e| {
            EncodeError::Encoding(format!("Scaler failed: {}", e))
        })?;

        nv12_avframe.set_pts(Some(frame.timestamp_us as i64));

        self.encoder.send_frame(&nv12_avframe).map_err(|e| {
            EncodeError::Encoding(format!("Encoder failed to receive frame: {}", e))
        })?;

        self.receive_and_send(stream_index, tx)?;

        self.frame_index += 1;
        Ok(())
    }

    pub fn flush(&mut self, stream_index: usize, tx: &Sender<EncodedPacket>) -> Result<(), EncodeError> {
        self.encoder.send_eof().map_err(|e| {
            EncodeError::Encoding(format!("Failed to send EOF: {}", e))
        })?;
        self.receive_and_send(stream_index, tx)?;
        Ok(())
    }

    fn receive_and_send(&mut self, stream_index: usize, tx: &Sender<EncodedPacket>) -> Result<(), EncodeError> {
        let mut packet = ffmpeg::Packet::empty();
        while self.encoder.receive_packet(&mut packet).is_ok() {
            // Need to create a new packet instance or clone, because `packet` is reused
            let mut p = packet.clone();
            p.set_stream(stream_index);
            if tx.send(EncodedPacket { stream_index, packet: p }).is_err() {
                break;
            }
        }
        Ok(())
    }
}
