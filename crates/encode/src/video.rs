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
        let codecs_to_try = ["h264_nvenc", "h264_qsv", "h264_amf", "libx264"];
        let mut codec_opt = None;

        for &c_name in &codecs_to_try {
            if let Some(codec) = ffmpeg::encoder::find_by_name(c_name) {
                codec_opt = Some((c_name, codec));
                break;
            }
        }

        let (c_name, codec) = codec_opt.ok_or_else(|| {
            EncodeError::Initialization("No H.264 encoder found on system".to_string())
        })?;

        info!("Selected Video Encoder: {}", c_name);

        let mut ctx = ffmpeg::codec::context::Context::new_with_codec(codec); // TODO(Phase 3): Context::new_with_codec was removed in ffmpeg-next. Update to Context::new() or from_parameters().
        let mut enc = ctx.encoder().video().map_err(|e| {
            EncodeError::Initialization(format!("Failed to get video encoder context: {}", e))
        })?;

        enc.set_width(width);
        enc.set_height(height);
        enc.set_format(ffmpeg::format::Pixel::NV12);
        enc.set_time_base((1, 1_000_000)); // microseconds
        // Add basic parameters for high-performance capture
        // We avoid b-frames for low latency
        enc.set_max_b_frames(0);

        let mut dict = ffmpeg::Dictionary::new();
        if c_name == "h264_nvenc" {
            dict.set("preset", "p1"); // fastest
            dict.set("tune", "ull");  // ultra low latency
        } else if c_name == "libx264" {
            dict.set("preset", "ultrafast");
            dict.set("tune", "zerolatency");
        }

        let encoder = enc.open_with(dict).map_err(|e| {
            EncodeError::Initialization(format!("Failed to open encoder {}: {}", c_name, e))
        })?;

        // Setup software scaler (BGRA -> NV12)
        let scaler = ffmpeg::software::scaling::Context::get(
            ffmpeg::format::Pixel::BGRA,
            width,
            height,
            ffmpeg::format::Pixel::NV12,
            width,
            height,
            ffmpeg::software::scaling::flag::Flags::FAST_BILINEAR,
        ).map_err(|e| {
            EncodeError::Initialization(format!("Failed to create scaler: {}", e))
        })?;

        Ok(Self {
            encoder,
            scaler,
            frame_index: 0,
            width,
            height,
        })
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
