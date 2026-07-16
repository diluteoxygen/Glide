use capture_core::{AudioCapturer, AudioFrame, AudioTrack, Frame, VideoCapturer};
use crossbeam_channel::bounded;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread;
use tracing::info;

#[cfg(target_os = "windows")]
use capture_windows::{audio::WasapiCapturer, DxgiCapturer};

#[cfg(target_os = "linux")]
use capture_linux::{audio::PipeWireAudioCapturer, PipeWireCapturer};

pub struct PipelineConfig {
    pub output_file: String,
    pub is_otf: bool,
    pub no_overlay: bool,
    pub record_system_audio: bool,
    pub selected_mic: Option<String>,
}

pub struct PipelineHandle {
    stop_signal: Arc<AtomicBool>,
}

impl PipelineHandle {
    pub fn start(config: PipelineConfig) -> Result<Self, Box<dyn std::error::Error>> {
        info!("--- STARTING RECORDING PIPELINE ---");

        #[cfg(target_os = "windows")]
        let mut vid_cap = DxgiCapturer::new().map_err(|e| e.to_string())?;
        
        #[cfg(target_os = "windows")]
        let sys_cap_opt = if config.record_system_audio {
            Some(WasapiCapturer::new(AudioTrack::SystemLoopback).map_err(|e| e.to_string())?)
        } else { None };
        
        #[cfg(target_os = "windows")]
        let mut mic_cap = WasapiCapturer::new(AudioTrack::Microphone).map_err(|e| e.to_string())?;

        #[cfg(target_os = "linux")]
        let mut vid_cap = PipeWireCapturer::new().map_err(|e| e.to_string())?;
        
        #[cfg(target_os = "linux")]
        let sys_cap_opt = if config.record_system_audio {
            Some(PipeWireAudioCapturer::new(AudioTrack::SystemLoopback).map_err(|e| e.to_string())?)
        } else { None };
        
        #[cfg(target_os = "linux")]
        let mut mic_cap = PipeWireAudioCapturer::new(AudioTrack::Microphone).map_err(|e| e.to_string())?;

        let (tx_vid, rx_vid) = crossbeam_channel::bounded::<Frame>(60);
        let (tx_sys, rx_sys) = crossbeam_channel::bounded::<AudioFrame>(500);
        let (tx_mic, rx_mic) = crossbeam_channel::bounded::<AudioFrame>(500);
        let (tx_mux, rx_mux) = crossbeam_channel::bounded::<encode::EncodedPacket>(5000);
        let (tx_params, rx_params) = crossbeam_channel::bounded::<encode::CodecParameters>(1);

        #[allow(unused_assignments)]
        let mut start_qpc_us = u64::MAX;
        #[cfg(target_os = "windows")]
        {
            let mut qpf = 0i64;
            unsafe { windows::Win32::System::Performance::QueryPerformanceFrequency(&mut qpf).unwrap(); }
            let mut qpc = 0i64;
            unsafe { windows::Win32::System::Performance::QueryPerformanceCounter(&mut qpc).unwrap(); }
            start_qpc_us = (qpc as u64 * 1_000_000) / qpf as u64;
        }

        let stop = Arc::new(AtomicBool::new(false));
        let dropped_frames = Arc::new(AtomicU64::new(0));
        let start_time = Arc::new(AtomicU64::new(start_qpc_us));

        let is_otf = config.is_otf;
        let no_overlay = config.no_overlay;
        let output_file = config.output_file.clone();

        if is_otf {
            info!("OTF Mode Enabled. Checking Accessibility Keys...");
            if input_hooks::are_accessibility_keys_enabled() {
                tracing::error!("Cannot start OTF recording because Sticky Keys or Filter Keys are enabled. Please disable them in Windows Settings.");
                return Err("Accessibility keys enabled".into());
            }
        }

        // Spawn Muxer
        let muxer = mux::Muxer::new(rx_mux, rx_params, output_file.clone()).map_err(|e| e.to_string())?;
        let stop_mux = Arc::clone(&stop);
        let _mux_thread = thread::spawn(move || muxer.start(stop_mux));

        // OTF Pipeline Check
        let encoder_rx_vid = if is_otf {
            let (tx_vid_comp, rx_vid_comp) = crossbeam_channel::bounded::<Frame>(60);
            let event_rx = input_hooks::InputHook::start();
            
            let (tx_overlay, rx_overlay) = crossbeam_channel::bounded(60);
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
        let encoder = encode::Encoder::new(encoder_rx_vid, rx_sys, rx_mic, tx_mux, tx_params).map_err(|e| e.to_string())?;
        let stop_enc = Arc::clone(&stop);
        let _enc_thread = thread::spawn(move || encoder.start(stop_enc));

        // Spawn Capturers
        let stop_vid = Arc::clone(&stop);
        let start_vid = Arc::clone(&start_time);
        let _vid_thread = thread::spawn(move || vid_cap.start(tx_vid, stop_vid, dropped_frames, start_vid));

        if let Some(mut sys_cap) = sys_cap_opt {
            let stop_sys = Arc::clone(&stop);
            let start_sys = Arc::clone(&start_time);
            let _sys_thread = thread::spawn(move || sys_cap.start(tx_sys, stop_sys, start_sys));
        }

        let stop_mic = Arc::clone(&stop);
        let start_mic = Arc::clone(&start_time);
        let _mic_thread = thread::spawn(move || mic_cap.start(tx_mic, stop_mic, start_mic));

        // Spawn Event Tracker (Cursor & Mouse Events)
        let event_log_path = std::path::PathBuf::from(&output_file).with_extension("events.jsonl");
        let mut event_tracker = camera::EventTracker::start(event_log_path, Arc::clone(&stop), Arc::clone(&start_time))
            .map_err(|e| e.to_string())?;

        let stop_clone = Arc::clone(&stop);
        thread::spawn(move || {
            // Wait for stop signal, then stop event tracker
            while !stop_clone.load(Ordering::Relaxed) {
                thread::sleep(std::time::Duration::from_millis(100));
            }
            event_tracker.stop();
        });

        Ok(Self {
            stop_signal: stop,
        })
    }

    pub fn stop(&self) {
        info!("Signaling stop to pipeline...");
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}

impl Drop for PipelineHandle {
    fn drop(&mut self) {
        self.stop();
    }
}
