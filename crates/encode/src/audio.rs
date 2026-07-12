use capture_core::AudioFrame;
use crossbeam_channel::Sender;
use ffmpeg_next as ffmpeg;
use tracing::{info, warn};
use crate::{EncodeError, EncodedPacket};

pub struct AudioEncoder {
    encoder: ffmpeg::codec::encoder::audio::Encoder,
    resampler: Option<ffmpeg::software::resampling::Context>,
    last_sample_rate: u32,
    last_channels: u16,
    frame_size: usize,
    sample_buffer: Vec<f32>,
    pts_counter: i64,
}

impl AudioEncoder {
    pub fn new(target_sample_rate: u32, target_channels: u16) -> Result<Self, EncodeError> {
        let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::AAC).ok_or_else(|| {
            EncodeError::Initialization("AAC encoder not found".to_string())
        })?;

        let mut ctx = ffmpeg::codec::context::Context::new_with_codec(codec);
        let mut enc = ctx.encoder().audio().map_err(|e| {
            EncodeError::Initialization(format!("Failed to get audio encoder context: {}", e))
        })?;

        enc.set_format(ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed));
        enc.set_rate(target_sample_rate as i32);
        
        let ch_layout = ffmpeg::util::channel_layout::ChannelLayout::default(target_channels as i32);
        enc.set_channel_layout(ch_layout);
        
        enc.set_time_base((1, target_sample_rate as i32));

        let encoder = enc.open_as(codec).map_err(|e| {
            EncodeError::Initialization(format!("Failed to open AAC encoder: {}", e))
        })?;

        // Most AAC encoders use 1024 frames. If frame_size is 0, it means variable, but let's assume 1024.
        let frame_size = if encoder.frame_size() > 0 { encoder.frame_size() as usize } else { 1024 };

        Ok(Self {
            encoder,
            resampler: None,
            last_sample_rate: 0,
            last_channels: 0,
            frame_size,
            sample_buffer: Vec::new(),
            pts_counter: 0,
        })
    }

    pub fn encode(
        &mut self,
        frame: AudioFrame,
        stream_index: usize,
        tx: &Sender<EncodedPacket>,
    ) -> Result<(), EncodeError> {
        // Recreate resampler if input format changes
        if self.last_sample_rate != frame.sample_rate || self.last_channels != frame.channels {
            let in_layout = ffmpeg::util::channel_layout::ChannelLayout::default(frame.channels as i32);
            let out_layout = self.encoder.channel_layout();
            let in_fmt = ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed);
            let out_fmt = ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed);

            let resampler = ffmpeg::software::resampling::Context::get(
                in_fmt,
                in_layout,
                frame.sample_rate,
                out_fmt,
                out_layout,
                self.encoder.rate(),
            ).map_err(|e| {
                EncodeError::Initialization(format!("Failed to create audio resampler: {}", e))
            })?;

            self.resampler = Some(resampler);
            self.last_sample_rate = frame.sample_rate;
            self.last_channels = frame.channels;
        }

        let num_samples = frame.data.len() / (frame.channels as usize);
        
        let mut in_frame = ffmpeg::frame::Audio::new(
            ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed),
            num_samples,
            ffmpeg::util::channel_layout::ChannelLayout::default(frame.channels as i32),
        );
        in_frame.set_rate(frame.sample_rate);
        
        let in_data = in_frame.data_mut(0);
        let src_bytes = bytemuck::cast_slice(&frame.data);
        in_data[..src_bytes.len()].copy_from_slice(src_bytes);

        let mut resampled_frame = ffmpeg::frame::Audio::empty();
        if let Some(resampler) = &mut self.resampler {
            resampler.run(&in_frame, &mut resampled_frame).map_err(|e| {
                EncodeError::Encoding(format!("Resampler failed: {}", e))
            })?;
        } else {
            resampled_frame = in_frame;
        }

        // Push to buffer
        let out_data = resampled_frame.data(0);
        let out_floats: &[f32] = bytemuck::cast_slice(out_data);
        self.sample_buffer.extend_from_slice(out_floats);

        let channels = self.encoder.channels() as usize;
        let floats_per_frame = self.frame_size * channels;

        while self.sample_buffer.len() >= floats_per_frame {
            let chunk: Vec<f32> = self.sample_buffer.drain(..floats_per_frame).collect();
            
            let mut out_frame = ffmpeg::frame::Audio::new(
                ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed),
                self.frame_size,
                self.encoder.channel_layout(),
            );
            out_frame.set_rate(self.encoder.rate());
            out_frame.set_pts(Some(self.pts_counter));
            self.pts_counter += self.frame_size as i64;
            
            let out_data_mut = out_frame.data_mut(0);
            out_data_mut.copy_from_slice(bytemuck::cast_slice(&chunk));

            self.encoder.send_frame(&out_frame).map_err(|e| {
                EncodeError::Encoding(format!("Encoder failed to receive frame: {}", e))
            })?;

            self.receive_and_send(stream_index, tx)?;
        }

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
            let mut p = packet.clone();
            p.set_stream(stream_index);
            if tx.send(EncodedPacket { stream_index, packet: p }).is_err() {
                break;
            }
        }
        Ok(())
    }
}
