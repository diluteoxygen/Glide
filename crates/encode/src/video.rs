use capture_core::Frame;
use crossbeam_channel::Sender;
use ffmpeg_next as ffmpeg;

use crate::{EncodeError, EncodedPacket};

pub struct VideoEncoder {
    pub encoder: ffmpeg::codec::encoder::video::Encoder,
    scaler: ffmpeg::software::scaling::Context,
    frame_index: i64,
    width: u32,
    height: u32,
    pub time_base: ffmpeg::Rational,
    last_pts: i64,
}

impl VideoEncoder {
    pub fn new(width: u32, height: u32) -> Result<Self, EncodeError> {
        let encoder_names = [
            "h264_nvenc",
            "h264_qsv",
            "h264_amf",
            "h264_vaapi",
            "h264_mf", // Windows Media Foundation fallback
            "libx264",
        ];
        let mut selected_codec = None;
        let mut selected_name = "";

        for name in encoder_names {
            if let Some(c) = ffmpeg::encoder::find_by_name(name) {
                selected_codec = Some(c);
                selected_name = name;
                break;
            }
        }

        let codec = selected_codec
            .ok_or_else(|| EncodeError::Initialization("No suitable H264 encoder found".into()))?;

        tracing::info!("Selected video encoder: {}", selected_name);

        let context = ffmpeg::codec::context::Context::new();
        let mut encoder = context
            .encoder()
            .video()
            .map_err(|e| EncodeError::Initialization(format!("Failed to create video context: {}", e)))?;

        encoder.set_width(width);
        encoder.set_height(height);
        encoder.set_format(ffmpeg::format::Pixel::NV12);
        if selected_name == "h264_mf" {
            // Windows Media Foundation requires a standard framerate time base
            encoder.set_time_base((1, 60));
            encoder.set_frame_rate(Some((60, 1)));
        } else {
            encoder.set_time_base((1, 1_000_000)); // microsecond precision
        }
        encoder.set_bit_rate(8_000_000); // 8 Mbps
        
        let mut dict = ffmpeg::Dictionary::new();
        if selected_name == "libx264" {
            dict.set("preset", "ultrafast");
            dict.set("tune", "zerolatency");
        } else if selected_name == "h264_nvenc" {
            dict.set("preset", "p1");
            dict.set("tune", "ull");
        } else if selected_name == "h264_amf" {
            dict.set("usage", "lowlatency");
        }

        let encoder = encoder
            .open_as_with(codec, dict)
            .map_err(|e| EncodeError::Initialization(format!("Failed to open H264 encoder: {}", e)))?;

        let scaler = ffmpeg::software::scaling::Context::get(
            ffmpeg::format::Pixel::BGRA,
            width,
            height,
            ffmpeg::format::Pixel::NV12,
            width,
            height,
            ffmpeg::software::scaling::flag::Flags::FAST_BILINEAR,
        )
        .map_err(|e| EncodeError::Initialization(format!("Failed to create video scaler: {}", e)))?;

        Ok(Self {
            encoder,
            scaler,
            frame_index: 0,
            width,
            height,
            time_base: if selected_name == "h264_mf" {
                ffmpeg::Rational(1, 60)
            } else {
                ffmpeg::Rational(1, 1_000_000)
            },
            last_pts: -1,
        })
    }

    pub fn encode(
        &mut self,
        frame: Frame,
        stream_index: usize,
        tx: &Sender<EncodedPacket>,
    ) -> Result<(), EncodeError> {
        // Create an AVFrame for the raw BGRA data
        let _raw_avframe = ffmpeg::frame::Video::empty();
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
        self.scaler
            .run(&raw_avframe, &mut nv12_avframe)
            .map_err(|e| EncodeError::Encoding(format!("Scaler failed: {}", e)))?;

        let time_base = self.time_base;
        let mut pts = (frame.timestamp_us as i64 * time_base.denominator() as i64) / (time_base.numerator() as i64 * 1_000_000);
        
        if pts <= self.last_pts {
            pts = self.last_pts + 1;
        }
        self.last_pts = pts;

        nv12_avframe.set_pts(Some(pts));

        self.encoder.send_frame(&nv12_avframe).map_err(|e| {
            EncodeError::Encoding(format!("Encoder failed to receive frame: {}", e))
        })?;

        if std::env::var("GLIDE_SLOW_ENCODER").is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        self.receive_and_send(stream_index, tx)?;

        self.frame_index += 1;
        Ok(())
    }

    pub fn flush(
        &mut self,
        stream_index: usize,
        tx: &Sender<EncodedPacket>,
    ) -> Result<(), EncodeError> {
        self.encoder
            .send_eof()
            .map_err(|e| EncodeError::Encoding(format!("Failed to send EOF: {}", e)))?;
        self.receive_and_send(stream_index, tx)?;
        Ok(())
    }

    fn receive_and_send(
        &mut self,
        stream_index: usize,
        tx: &Sender<EncodedPacket>,
    ) -> Result<(), EncodeError> {
        let mut packet = ffmpeg::Packet::empty();
        while self.encoder.receive_packet(&mut packet).is_ok() {
            // Need to create a new packet instance or clone, because `packet` is reused
            let mut p = packet.clone();
            p.set_stream(stream_index);
            if tx
                .send(EncodedPacket {
                    stream_index,
                    packet: p,
                })
                .is_err()
            {
                break;
            }
        }
        Ok(())
    }
}
