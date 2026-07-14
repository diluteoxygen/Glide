use crossbeam_channel::Sender;
use encode::EncodedPacket;
use ffmpeg_next as ffmpeg;
use std::path::Path;

pub struct Decoder {
    input_ctx: ffmpeg::format::context::Input,
    video_stream_index: usize,
    audio_sys_stream_index: Option<usize>,
    audio_mic_stream_index: Option<usize>,
    video_decoder: ffmpeg::codec::decoder::Video,
    scaler: ffmpeg::software::scaling::Context,
}

impl Decoder {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        ffmpeg::init().map_err(|e| format!("FFmpeg init failed: {}", e))?;
        let mut input_ctx = ffmpeg::format::input(&path).map_err(|e| format!("Failed to open input: {}", e))?;

        let video_stream = input_ctx.streams().best(ffmpeg::media::Type::Video).ok_or("No video stream found")?;
        let video_stream_index = video_stream.index();
        let video_context = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            .map_err(|e| format!("Failed to get video context: {}", e))?;
        let video_decoder = video_context.decoder().video()
            .map_err(|e| format!("Failed to create video decoder: {}", e))?;

        // Find audio streams
        let mut audio_streams = input_ctx.streams().filter(|s| s.parameters().medium() == ffmpeg::media::Type::Audio);
        let audio_sys_stream_index = audio_streams.next().map(|s| s.index());
        let audio_mic_stream_index = audio_streams.next().map(|s| s.index());

        let scaler = ffmpeg::software::scaling::Context::get(
            video_decoder.format(),
            video_decoder.width(),
            video_decoder.height(),
            ffmpeg::format::Pixel::BGRA,
            video_decoder.width(),
            video_decoder.height(),
            ffmpeg::software::scaling::flag::Flags::FAST_BILINEAR,
        ).map_err(|e| format!("Failed to create scaler: {}", e))?;

        Ok(Self {
            input_ctx,
            video_stream_index,
            audio_sys_stream_index,
            audio_mic_stream_index,
            video_decoder,
            scaler,
        })
    }

    pub fn audio_sys_params(&self) -> Option<(ffmpeg::codec::Parameters, ffmpeg::Rational)> {
        self.audio_sys_stream_index.map(|idx| {
            let stream = self.input_ctx.stream(idx).unwrap();
            (stream.parameters(), stream.time_base())
        })
    }

    pub fn audio_mic_params(&self) -> Option<(ffmpeg::codec::Parameters, ffmpeg::Rational)> {
        self.audio_mic_stream_index.map(|idx| {
            let stream = self.input_ctx.stream(idx).unwrap();
            (stream.parameters(), stream.time_base())
        })
    }
    
    pub fn video_time_base(&self) -> ffmpeg::Rational {
        self.input_ctx.stream(self.video_stream_index).unwrap().time_base()
    }
    
    pub fn video_dimensions(&self) -> (u32, u32) {
        (self.video_decoder.width(), self.video_decoder.height())
    }

    pub fn run(
        &mut self,
        mut video_callback: impl FnMut(ffmpeg::frame::Video, i64) -> Result<(), String>,
        audio_tx: &Sender<EncodedPacket>,
    ) -> Result<(), String> {
        for (stream, mut packet) in self.input_ctx.packets() {
            let index = stream.index();
            if index == self.video_stream_index {
                self.video_decoder.send_packet(&packet).map_err(|e| format!("Decode error: {}", e))?;
                let mut decoded = ffmpeg::frame::Video::empty();
                while self.video_decoder.receive_frame(&mut decoded).is_ok() {
                    let mut bgra_frame = ffmpeg::frame::Video::new(
                        ffmpeg::format::Pixel::BGRA,
                        decoded.width(),
                        decoded.height(),
                    );
                    self.scaler.run(&decoded, &mut bgra_frame).map_err(|e| format!("Scale error: {}", e))?;
                    
                    let pts = decoded.pts().unwrap_or(0);
                    video_callback(bgra_frame, pts)?;
                }
            } else if Some(index) == self.audio_sys_stream_index || Some(index) == self.audio_mic_stream_index {
                let out_index = if Some(index) == self.audio_sys_stream_index { 1 } else { 2 };
                packet.set_stream(out_index);
                let _ = audio_tx.send(EncodedPacket {
                    stream_index: out_index,
                    packet,
                });
            }
        }
        
        // Flush decoder
        self.video_decoder.send_eof().map_err(|e| format!("Flush error: {}", e))?;
        let mut decoded = ffmpeg::frame::Video::empty();
        while self.video_decoder.receive_frame(&mut decoded).is_ok() {
            let mut bgra_frame = ffmpeg::frame::Video::new(
                ffmpeg::format::Pixel::BGRA,
                decoded.width(),
                decoded.height(),
            );
            self.scaler.run(&decoded, &mut bgra_frame).map_err(|e| format!("Scale error: {}", e))?;
            let pts = decoded.pts().unwrap_or(0);
            video_callback(bgra_frame, pts)?;
        }
        
        Ok(())
    }
}
