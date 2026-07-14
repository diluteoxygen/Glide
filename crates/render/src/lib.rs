pub mod compositor;
pub mod decode;

use camera::solver::VirtualCamera;
use camera::event_log::LogEntry;
use capture_core::{Frame, PixelFormat};
use crossbeam_channel::unbounded;
use encode::video::VideoEncoder;
use mux::Muxer;
use std::path::Path;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};

pub fn render_video<P1: AsRef<Path>, P2: AsRef<Path>, P3: AsRef<Path>>(
    input_video_path: P1,
    events_log_path: P2,
    output_video_path: P3,
) -> Result<(), String> {
    let mut decoder = decode::Decoder::new(input_video_path)?;
    let (width, height) = decoder.video_dimensions();
    let video_time_base = decoder.video_time_base();

    // Load events
    let content = std::fs::read_to_string(&events_log_path).map_err(|e| format!("Failed to read log: {}", e))?;
    let mut events = Vec::new();
    for line in content.lines() {
        if let Ok(entry) = serde_json::from_str::<LogEntry>(line) {
            events.push(entry);
        }
    }
    
    // Set up channels
    let (packet_tx, packet_rx) = unbounded();
    let (params_tx, params_rx) = unbounded();

    // Start muxer in background
    let stop_muxer = Arc::new(AtomicBool::new(false));
    let muxer = Muxer::new(packet_rx, params_rx, output_video_path.as_ref().to_string_lossy().to_string())
        .map_err(|e| format!("Failed to create muxer: {}", e))?;
    let stop_muxer_clone = stop_muxer.clone();
    let mux_thread = std::thread::spawn(move || {
        muxer.start(stop_muxer_clone)
    });

    // Set up encoder
    let mut encoder = VideoEncoder::new(width, height)
        .map_err(|e| format!("Failed to create encoder: {}", e))?;
        
    let encoder_params = (&encoder.encoder).into();

    let mut audio_sys = ffmpeg_next::codec::Parameters::new();
    let mut audio_sys_tb = ffmpeg_next::Rational(1, 48000);
    if let Some((p, tb)) = decoder.audio_sys_params() {
        audio_sys = p;
        audio_sys_tb = tb;
    }
    
    let mut audio_mic = ffmpeg_next::codec::Parameters::new();
    let mut audio_mic_tb = ffmpeg_next::Rational(1, 48000);
    if let Some((p, tb)) = decoder.audio_mic_params() {
        audio_mic = p;
        audio_mic_tb = tb;
    }

    let codec_params = encode::CodecParameters {
        video: encoder_params,
        video_time_base: encoder.time_base,
        audio_sys,
        audio_sys_time_base: audio_sys_tb,
        audio_mic,
        audio_mic_time_base: audio_mic_tb,
    };
    params_tx.send(codec_params).unwrap();

    let mut compositor = compositor::Compositor::new(width, height);
    let mut virtual_camera = VirtualCamera::new((width / 2) as f32, (height / 2) as f32);

    let mut last_pts_us = 0;

    let res = decoder.run(
        |frame, pts| {
            // pts is in video_time_base. convert to microseconds for the solver.
            let mut time_us = (pts as i64 * video_time_base.numerator() as i64 * 1_000_000)
                / video_time_base.denominator() as i64;
            let mut time_us = time_us.max(0) as u64;

            if time_us <= last_pts_us && last_pts_us > 0 {
                time_us = last_pts_us + 1;
            }

            let dt_seconds = (time_us - last_pts_us) as f32 / 1_000_000.0;
            last_pts_us = time_us;

            let (target_x, target_y, target_zoom) = virtual_camera.compute_target(time_us, &events);
            
            // Limit dt to avoid physics explosion on first frame
            let step_dt = dt_seconds.min(0.1).max(0.001);
            let (cx, cy, zoom) = virtual_camera.step(step_dt, target_x, target_y, target_zoom);

            let bgra_data = frame.data(0);
            let stride = frame.stride(0) as u32;
            let rendered_bgra = compositor.render_frame(bgra_data, stride, zoom, cx, cy);

            let mut out_frame = Frame {
                data: vec![0; (width * height * 4) as usize],
                format: PixelFormat::Bgra,
                width,
                height,
                timestamp_us: time_us,
            };
            out_frame.data.copy_from_slice(&rendered_bgra);

            encoder.encode(out_frame, 0, &packet_tx).map_err(|e| format!("Encode error: {}", e))?;
            
            Ok(())
        },
        &packet_tx,
    );

    encoder.flush(0, &packet_tx).map_err(|e| format!("Failed to flush encoder: {}", e))?;
    
    stop_muxer.store(true, Ordering::Relaxed);
    mux_thread.join().unwrap().map_err(|e| format!("Muxer error: {:?}", e))?;

    res
}
