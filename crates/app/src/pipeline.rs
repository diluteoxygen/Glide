use capture_core::{AudioCapturer, AudioFrame, AudioTrack, Frame, VideoCapturer};
use crossbeam_channel::bounded;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread;
use std::path::PathBuf;
use tracing::info;

#[cfg(target_os = "windows")]
use capture_windows::{audio::WasapiCapturer, DxgiCapturer};

#[cfg(target_os = "linux")]
use capture_linux::{audio::PipeWireAudioCapturer, PipeWireCapturer};

pub struct PipelineHandle {
    pub stop_signal: Arc<AtomicBool>,
}

pub fn start_recording(
    output_dir: &str,
    is_otf: bool,
    no_overlay: bool,
) -> Result<PipelineHandle, String> {
    info!("Starting recording pipeline in {}", output_dir);

    // Create unique filename
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let output_file = PathBuf::from(output_dir).join(format!("record_{}.mkv", timestamp));

    #[cfg(target_os = "windows")]
    let mut vid_cap = DxgiCapturer::new().map_err(|e| format!("DXGI init failed: {}", e))?;
    #[cfg(target_os = "windows")]
    let (mut sys_cap, mut mic_cap) = (
        WasapiCapturer::new(AudioTrack::SystemLoopback).map_err(|e| format!("System audio init failed: {}", e))?,
        WasapiCapturer::new(AudioTrack::Microphone).map_err(|e| format!("Mic audio init failed: {}", e))?,
    );

    #[cfg(target_os = "linux")]
    let mut vid_cap = PipeWireCapturer::new().map_err(|e| format!("PipeWire init failed: {}", e))?;
    #[cfg(target_os = "linux")]
    let (mut sys_cap, mut mic_cap) = (
        PipeWireAudioCapturer::new(AudioTrack::SystemLoopback).map_err(|e| format!("System audio init failed: {}", e))?,
        PipeWireAudioCapturer::new(AudioTrack::Microphone).map_err(|e| format!("Mic audio init failed: {}", e))?,
    );

    let (tx_vid, rx_vid) = bounded::<Frame>(60);
    let (tx_sys, rx_sys) = bounded::<AudioFrame>(500);
    let (tx_mic, rx_mic) = bounded::<AudioFrame>(500);
    let (tx_mux, rx_mux) = bounded::<encode::EncodedPacket>(5000);
    let (tx_params, rx_params) = bounded::<encode::CodecParameters>(1);

    let stop = Arc::new(AtomicBool::new(false));
    let dropped_frames = Arc::new(AtomicU64::new(0));
    let start_time = Arc::new(AtomicU64::new(u64::MAX));

    if is_otf {
        info!("OTF Mode Enabled. Checking Accessibility Keys...");
        if input_hooks::are_accessibility_keys_enabled() {
            return Err("Cannot start OTF recording because Sticky Keys or Filter Keys are enabled. Please disable them in Windows Settings.".to_string());
        }
    }

    // Spawn Muxer
    let muxer = mux::Muxer::new(rx_mux, rx_params, output_file.to_string_lossy().to_string())
        .map_err(|e| format!("Muxer init failed: {}", e))?;
    let stop_mux = Arc::clone(&stop);
    thread::spawn(move || muxer.start(stop_mux));

    // OTF Pipeline Check
    let encoder_rx_vid = if is_otf {
        let (tx_vid_comp, rx_vid_comp) = bounded::<Frame>(60);
        let event_rx = input_hooks::InputHook::start();
        
        let (tx_overlay, rx_overlay) = bounded(60);
        if !no_overlay {
            otf_overlay::OtfOverlay::start(rx_overlay, Arc::clone(&stop));
        }

        info!("Spawning Live Compositor...");
        otf_compositor::LiveCompositor::start(
            rx_vid,
            tx_vid_comp,
            event_rx,
            tx_overlay,
            Arc::clone(&stop)
        );
        rx_vid_comp
    } else {
        rx_vid
    };

    // Spawn Encoder
    let encoder = encode::Encoder::new(encoder_rx_vid, rx_sys, rx_mic, tx_mux, tx_params)
        .map_err(|e| format!("Encoder init failed: {}", e))?;
    let stop_enc = Arc::clone(&stop);
    thread::spawn(move || encoder.start(stop_enc));

    // Spawn Capturers
    let stop_vid = Arc::clone(&stop);
    let start_vid = Arc::clone(&start_time);
    thread::spawn(move || vid_cap.start(tx_vid, stop_vid, dropped_frames, start_vid));

    let stop_sys = Arc::clone(&stop);
    let start_sys = Arc::clone(&start_time);
    thread::spawn(move || sys_cap.start(tx_sys, stop_sys, start_sys));

    let stop_mic = Arc::clone(&stop);
    let start_mic = Arc::clone(&start_time);
    thread::spawn(move || mic_cap.start(tx_mic, stop_mic, start_mic));

    // Spawn Event Tracker (Cursor & Mouse Events)
    let event_log_path = output_file.with_extension("events.jsonl");
    let event_tracker = camera::EventTracker::start(event_log_path, Arc::clone(&stop), Arc::clone(&start_time))
        .map_err(|e| format!("Failed to start event tracker: {}", e))?;
        
    // Keep event tracker alive so it keeps recording events until stop is signaled
    // Actually the event tracker runs on its own thread, we just need to drop it eventually or let it exit
    // Since EventTracker::start spawns a thread internally, we just need to hold it?
    // Wait, EventTracker has a stop() method, but the loop stops when `stop` is true.
    // So we don't strictly need to hold the object if it just joins in Drop.
    // BUT EventTracker drops immediately if not kept! Wait, it has a Drop impl that waits for the thread?
    // Let's store event_tracker in a thread that just waits for stop_signal to avoid blocking the UI thread on drop.
    let stop_tracker_wait = Arc::clone(&stop);
    thread::spawn(move || {
        let mut tracker = event_tracker;
        while !stop_tracker_wait.load(Ordering::Relaxed) {
            thread::sleep(std::time::Duration::from_millis(100));
        }
        tracker.stop();
    });

    Ok(PipelineHandle {
        stop_signal: stop,
    })
}
