use capture_core::AudioFrame;
use crossbeam_channel::Sender;
use ffmpeg_next as ffmpeg;

use crate::{EncodeError, EncodedPacket};

pub struct AudioEncoder {
    pub(crate) encoder: ffmpeg::codec::encoder::audio::Encoder,
    resampler: Option<ffmpeg::software::resampling::Context>,
    last_sample_rate: u32,
    last_channels: u16,
    frame_size: usize,
    /// Per-channel sample buffers. sample_buffers[ch] holds f32 samples for that channel.
    sample_buffers: Vec<Vec<f32>>,
    anchor_pts: Option<u64>,
    pub(crate) time_base: ffmpeg::Rational,
    out_channels: usize,
}

impl AudioEncoder {
    pub fn new(target_sample_rate: u32, target_channels: u16) -> Result<Self, EncodeError> {
        let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::AAC)
            .ok_or_else(|| EncodeError::Initialization("AAC codec not found".into()))?;

        let context = ffmpeg::codec::context::Context::new();
        let mut encoder = context
            .encoder()
            .audio()
            .map_err(|e| EncodeError::Initialization(format!("Failed to create audio context: {}", e)))?;

        encoder.set_rate(target_sample_rate as i32);
        let channel_layout = ffmpeg::util::channel_layout::ChannelLayout::default(target_channels as i32);
        encoder.set_channel_layout(channel_layout);
        encoder.set_channels(channel_layout.channels());
        // AAC strictly requires FLTP (Float Planar)
        encoder.set_format(ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar));
        encoder.set_bit_rate(128_000);
        let time_base = ffmpeg::Rational(1, target_sample_rate as i32);
        encoder.set_time_base(time_base);

        let encoder = encoder
            .open_as(codec)
            .map_err(|e| EncodeError::Initialization(format!("Failed to open AAC encoder: {}", e)))?;

        let frame_size = encoder.frame_size() as usize;
        let out_channels = encoder.channels() as usize;

        Ok(Self {
            encoder,
            resampler: None,
            last_sample_rate: 0,
            last_channels: 0,
            frame_size,
            sample_buffers: (0..out_channels).map(|_| Vec::new()).collect(),
            anchor_pts: None,
            time_base,
            out_channels,
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
            let in_layout =
                ffmpeg::util::channel_layout::ChannelLayout::default(frame.channels as i32);
            let out_layout = self.encoder.channel_layout();
            let in_fmt = ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed);
            let out_fmt = self.encoder.format();

            let resampler = ffmpeg::software::resampling::Context::get(
                in_fmt,
                in_layout,
                frame.sample_rate,
                out_fmt,
                out_layout,
                self.encoder.rate(),
            )
            .map_err(|e| {
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
            resampler
                .run(&in_frame, &mut resampled_frame)
                .map_err(|e| EncodeError::Encoding(format!("Resampler failed: {}", e)))?;
        } else {
            resampled_frame = in_frame;
        }

        // The resampled frame is in planar format: each channel is a separate plane.
        // Read each plane and push to the corresponding per-channel buffer.
        for ch in 0..self.out_channels {
            // Use strongly-typed `plane::<f32>(ch)` which computes size based on nb_samples
            // instead of data(ch) which incorrectly uses linesize[ch] that might be 0.
            let plane_floats = resampled_frame.plane::<f32>(ch);
            if !plane_floats.is_empty() {
                if ch == 0 {
                    let incoming_pts = (frame.timestamp_us * self.encoder.rate() as u64) / 1_000_000;
                    
                    if let Some(current_anchor) = self.anchor_pts {
                        let expected_pts = current_anchor + self.sample_buffers[0].len() as u64;
                        let tolerance = (self.encoder.rate() / 10) as u64; // 100ms
                        
                        let gap = if incoming_pts > expected_pts {
                            incoming_pts - expected_pts
                        } else {
                            expected_pts - incoming_pts
                        };
                        
                        if gap > tolerance {
                            let gap_ms = (gap * 1000) / self.encoder.rate() as u64;
                            let tol_ms = (tolerance * 1000) / self.encoder.rate() as u64;
                            tracing::warn!("Audio PTS gap detected: incoming {}, expected {}, gap {} samples ({}ms) > tolerance {} samples ({}ms). Re-anchoring.", 
                                incoming_pts, expected_pts, gap, gap_ms, tolerance, tol_ms);
                            self.anchor_pts = Some(incoming_pts.saturating_sub(self.sample_buffers[0].len() as u64));
                        }
                    } else {
                        self.anchor_pts = Some(incoming_pts);
                    }
                }
                self.sample_buffers[ch].extend_from_slice(plane_floats);
            }
        }

        // Build output frames once we have enough samples in every channel
        while self.sample_buffers[0].len() >= self.frame_size {
            let mut out_frame = ffmpeg::frame::Audio::new(
                self.encoder.format(),
                self.frame_size,
                self.encoder.channel_layout(),
            );
            // Explicitly set channels because Audio::new only sets channel_layout, 
            // which FFmpeg 6.0+ might misinterpret without channels explicitly set.
            out_frame.set_channels(self.out_channels as u16);
            out_frame.set_rate(self.encoder.rate());
            out_frame.set_pts(self.anchor_pts.map(|v| v as i64));

            if let Some(pts) = &mut self.anchor_pts {
                *pts += self.frame_size as u64;
            }

            // Write each channel's samples to its plane
            for ch in 0..self.out_channels {
                let samples: Vec<f32> = self.sample_buffers[ch].drain(..self.frame_size).collect();
                let plane = out_frame.plane_mut::<f32>(ch);
                plane[..samples.len()].copy_from_slice(&samples);
            }

            self.encoder.send_frame(&out_frame).map_err(|e| {
                EncodeError::Encoding(format!("Encoder failed to receive frame: {}", e))
            })?;

            self.receive_and_send(stream_index, tx)?;
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use capture_core::AudioTrack;
    use crossbeam_channel::bounded;

    #[test]
    fn test_audio_pts_gap() {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);

        ffmpeg::init().unwrap();

        let mut encoder = AudioEncoder::new(48000, 2).unwrap();
        let (tx, _rx) = bounded(10000);

        let samples_per_chunk = 960; // 20ms at 48kHz
        let floats_per_chunk = samples_per_chunk * 2;
        let mut current_us: u64 = 0;

        for i in 0..500 { // 10 seconds total (500 * 20ms)
            let frame = AudioFrame {
                data: vec![0.0f32; floats_per_chunk],
                sample_rate: 48000,
                channels: 2,
                track: AudioTrack::SystemLoopback,
                timestamp_us: current_us,
            };

            encoder.encode(frame, 0, &tx).unwrap();
            current_us += 20_000;

            if i == 250 {
                current_us += 105_000; // Simulated dropped frames!
                tracing::info!("--- SIMULATED GAP OF 105ms (DROPPED FRAMES) ---");
            }
        }
    }
}
