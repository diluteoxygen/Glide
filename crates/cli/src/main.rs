use capture_core::{AudioCapturer, AudioFrame, AudioTrack, Frame, VideoCapturer};
use crossbeam_channel::bounded;
use std::env;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::System;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[cfg(target_os = "windows")]
use capture_windows::{audio::WasapiCapturer, DxgiCapturer};

#[cfg(target_os = "linux")]
use capture_linux::{audio::PipeWireAudioCapturer, PipeWireCapturer};

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting Glide CLI");
    info!("Platform: {}", env::consts::OS);

    let args: Vec<String> = env::args().collect();
    
    if args.contains(&"--audio-test".to_string()) {
        run_audio_test();
    } else if args.contains(&"--video-test".to_string()) {
        let dump_frame = args.contains(&"--dump-frame".to_string());
        run_video_test(dump_frame);
    } else {
        run_full_pipeline();
    }
}

fn run_full_pipeline() {
    info!("--- FULL RECORDING PIPELINE ---");

    #[cfg(target_os = "windows")]
    let mut vid_cap = DxgiCapturer::new().expect("Failed to initialize DXGI capturer");
    #[cfg(target_os = "windows")]
    let (mut sys_cap, mut mic_cap) = (
        WasapiCapturer::new(AudioTrack::SystemLoopback).expect("Failed to init system audio"),
        WasapiCapturer::new(AudioTrack::Microphone).expect("Failed to init mic audio")
    );

    #[cfg(target_os = "linux")]
    let mut vid_cap = PipeWireCapturer::new().expect("Failed to initialize PipeWire capturer");
    #[cfg(target_os = "linux")]
    let (mut sys_cap, mut mic_cap) = (
        PipeWireAudioCapturer::new(AudioTrack::SystemLoopback).expect("Failed to init system audio"),
        PipeWireAudioCapturer::new(AudioTrack::Microphone).expect("Failed to init mic audio")
    );

    let (tx_vid, rx_vid) = crossbeam_channel::bounded::<Frame>(5);
    let (tx_sys, rx_sys) = crossbeam_channel::bounded::<AudioFrame>(100);
    let (tx_mic, rx_mic) = crossbeam_channel::bounded::<AudioFrame>(100);
    let (tx_mux, rx_mux) = crossbeam_channel::bounded::<encode::EncodedPacket>(1000);

    let stop = Arc::new(AtomicBool::new(false));
    let dropped_frames = Arc::new(AtomicU64::new(0));

    // Spawn Muxer
    let muxer = mux::Muxer::new(rx_mux, "output.mkv".to_string()).expect("Failed to init muxer");
    let stop_mux = Arc::clone(&stop);
    let mux_thread = thread::spawn(move || muxer.start(stop_mux));

    // Spawn Encoder
    let encoder = encode::Encoder::new(rx_vid, rx_sys, rx_mic, tx_mux).expect("Failed to init encoder");
    let stop_enc = Arc::clone(&stop);
    let enc_thread = thread::spawn(move || encoder.start(stop_enc));

    // Spawn Capturers
    let stop_vid = Arc::clone(&stop);
    let vid_thread = thread::spawn(move || vid_cap.start(tx_vid, stop_vid, dropped_frames));
    
    let stop_sys = Arc::clone(&stop);
    let sys_thread = thread::spawn(move || sys_cap.start(tx_sys, stop_sys));
    
    let stop_mic = Arc::clone(&stop);
    let mic_thread = thread::spawn(move || mic_cap.start(tx_mic, stop_mic));

    info!("Recording for 10 seconds...");
    thread::sleep(Duration::from_secs(10));

    info!("Signaling stop to all threads...");
    stop.store(true, Ordering::Relaxed);

    vid_thread.join().unwrap().unwrap();
    sys_thread.join().unwrap().unwrap();
    mic_thread.join().unwrap().unwrap();
    enc_thread.join().unwrap().unwrap();
    mux_thread.join().unwrap().unwrap();

    info!("Full pipeline closed successfully. Output saved to output.mkv");
}

fn run_audio_test() {
    info!("--- AUDIO TEST ---");

    #[cfg(target_os = "windows")]
    let (mut sys_cap, mut mic_cap) = (
        WasapiCapturer::new(AudioTrack::SystemLoopback).expect("Failed to init system audio"),
        WasapiCapturer::new(AudioTrack::Microphone).expect("Failed to init mic audio")
    );

    #[cfg(target_os = "linux")]
    let (mut sys_cap, mut mic_cap) = (
        PipeWireAudioCapturer::new(AudioTrack::SystemLoopback).expect("Failed to init system audio"),
        PipeWireAudioCapturer::new(AudioTrack::Microphone).expect("Failed to init mic audio")
    );

    let (tx, rx) = bounded::<AudioFrame>(100);
    let stop = Arc::new(AtomicBool::new(false));

    let tx_sys = tx.clone();
    let stop_sys = Arc::clone(&stop);
    let sys_thread = thread::spawn(move || sys_cap.start(tx_sys, stop_sys));

    let tx_mic = tx.clone();
    let stop_mic = Arc::clone(&stop);
    let mic_thread = thread::spawn(move || mic_cap.start(tx_mic, stop_mic));

    let run_duration = Duration::from_secs(10);
    let start_time = Instant::now();

    let mut sys_writer = None;
    let mut mic_writer = None;

    info!("Running audio capture for 10 seconds...");

    let mut sys_frames = 0;
    let mut mic_frames = 0;

    while start_time.elapsed() < run_duration {
        while let Ok(frame) = rx.try_recv() {
            match frame.track {
                AudioTrack::SystemLoopback => {
                    let writer = sys_writer.get_or_insert_with(|| {
                        let spec = hound::WavSpec {
                            channels: frame.channels,
                            sample_rate: frame.sample_rate,
                            bits_per_sample: 32,
                            sample_format: hound::SampleFormat::Float,
                        };
                        hound::WavWriter::create("system.wav", spec).expect("Failed to create system.wav")
                    });
                    for &sample in &frame.data {
                        writer.write_sample(sample).unwrap();
                    }
                    sys_frames += 1;
                }
                AudioTrack::Microphone => {
                    let writer = mic_writer.get_or_insert_with(|| {
                        let spec = hound::WavSpec {
                            channels: frame.channels,
                            sample_rate: frame.sample_rate,
                            bits_per_sample: 32,
                            sample_format: hound::SampleFormat::Float,
                        };
                        hound::WavWriter::create("mic.wav", spec).expect("Failed to create mic.wav")
                    });
                    for &sample in &frame.data {
                        writer.write_sample(sample).unwrap();
                    }
                    mic_frames += 1;
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    info!("Signaling stop to audio capture threads...");
    stop.store(true, Ordering::Relaxed);

    // Drain
    while rx.try_recv().is_ok() {}

    if let Some(writer) = sys_writer {
        writer.finalize().unwrap();
    }
    if let Some(writer) = mic_writer {
        writer.finalize().unwrap();
    }

    let _ = sys_thread.join();
    let _ = mic_thread.join();

    info!("--- Audio Test Results ---");
    info!("System Loopback packets written: {}", sys_frames);
    info!("Microphone packets written:      {}", mic_frames);
}

fn run_video_test(dump_frame: bool) {
    if dump_frame {
        info!("--dump-frame flag detected: will save the first captured frame to dump.png");
    }

    // Initialize the appropriate capturer
    #[cfg(target_os = "windows")]
    let mut capturer = DxgiCapturer::new().expect("Failed to initialize DXGI capturer");

    #[cfg(target_os = "linux")]
    let mut capturer = PipeWireCapturer::new().expect("Failed to initialize PipeWire capturer");

    let (tx, rx) = bounded::<Frame>(5);
    let stop = Arc::new(AtomicBool::new(false));
    let dropped_frames = Arc::new(AtomicU64::new(0));

    let stop_clone = Arc::clone(&stop);
    let dropped_clone = Arc::clone(&dropped_frames);

    // Spawn capture thread
    let capture_thread = thread::spawn(move || capturer.start(tx, stop_clone, dropped_clone));

    // Initialize sysinfo to monitor CPU usage
    let mut sys = System::new_all();
    let pid = sysinfo::get_current_pid().expect("Failed to get current PID");

    let mut received_frames = 0;
    let run_duration = Duration::from_secs(10);
    let start_time = Instant::now();
    let mut last_log = Instant::now();

    let mut last_received = 0;
    let mut last_dropped = 0;

    let mut frame_dumped = false;

    info!("Running capture for 10 seconds...");

    while start_time.elapsed() < run_duration {
        while let Ok(frame) = rx.try_recv() {
            received_frames += 1;

            if dump_frame && !frame_dumped {
                info!("Dumping first frame to dump.png...");
                let mut rgba = vec![0u8; frame.data.len()];
                for (src, dst) in frame.data.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
                    dst[0] = src[2]; // R
                    dst[1] = src[1]; // G
                    dst[2] = src[0]; // B
                    dst[3] = 255;    // A (force opaque)
                }
                if let Err(e) = image::save_buffer("dump.png", &rgba, frame.width, frame.height, image::ColorType::Rgba8) {
                    tracing::error!("Failed to save dump.png: {}", e);
                } else {
                    info!("Successfully saved dump.png");
                }
                frame_dumped = true;
            }
        }

        // Log stats every second
        if last_log.elapsed() >= Duration::from_secs(1) {
            sys.refresh_processes();
            let cpu_usage = if let Some(process) = sys.process(pid) {
                process.cpu_usage()
            } else {
                0.0
            };

            let dropped = dropped_frames.load(Ordering::Relaxed);
            let recent_received = received_frames - last_received;
            let recent_dropped = dropped - last_dropped;
            let recent_total = recent_received + recent_dropped;
            let elapsed_secs = last_log.elapsed().as_secs_f64();
            let fps = recent_total as f64 / elapsed_secs;

            info!(
                "Stats: {:.1} FPS | Received: {} | Dropped: {} | CPU: {:.1}%",
                fps, recent_received, recent_dropped, cpu_usage
            );

            last_received = received_frames;
            last_dropped = dropped;
            last_log = Instant::now();
        }

        thread::sleep(Duration::from_millis(5));
    }

    info!("Signaling stop to capture thread...");
    stop.store(true, Ordering::Relaxed);

    // Drain channel to unblock capturer if it's waiting on a full channel
    while rx.try_recv().is_ok() {}

    match capture_thread.join() {
        Ok(Ok(_)) => info!("Capture thread exited cleanly."),
        Ok(Err(e)) => info!("Capture thread returned error: {}", e),
        Err(_) => info!("Capture thread panicked."),
    }

    let total_dropped = dropped_frames.load(Ordering::Relaxed);
    let total_fps = (received_frames + total_dropped) as f64 / 10.0;
    info!("--- Final Results ---");
    info!("Total Received: {}", received_frames);
    info!("Total Dropped:  {}", total_dropped);
    info!("Average FPS:    {:.1}", total_fps);
}
