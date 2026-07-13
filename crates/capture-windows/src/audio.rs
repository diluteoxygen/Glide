use capture_core::{AudioCapturer, AudioFrame, AudioTrack, CaptureError};
use crossbeam_channel::Sender;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use windows::Win32::Media::Audio::{
    eCapture, eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
    MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};

pub struct WasapiCapturer {
    track: AudioTrack,
    audio_client: IAudioClient,
    capture_client: IAudioCaptureClient,
    sample_rate: u32,
    channels: u16,
}

unsafe impl Send for WasapiCapturer {}

impl WasapiCapturer {
    pub fn new(track: AudioTrack) -> Result<Self, CaptureError> {
        unsafe {
            // Ensure COM is initialized for this thread
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| {
                    CaptureError::Initialization(format!(
                        "Failed to create device enumerator: {}",
                        e
                    ))
                })?;

            let data_flow = match track {
                AudioTrack::SystemLoopback => eRender,
                AudioTrack::Microphone => eCapture,
            };

            let device = enumerator
                .GetDefaultAudioEndpoint(data_flow, eConsole)
                .map_err(|e| {
                    CaptureError::Initialization(format!(
                        "Failed to get default audio endpoint: {}",
                        e
                    ))
                })?;

            let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|e| {
                CaptureError::Initialization(format!("Failed to activate IAudioClient: {}", e))
            })?;

            let mix_format_ptr = audio_client.GetMixFormat().map_err(|e| {
                CaptureError::Initialization(format!("Failed to get mix format: {}", e))
            })?;

            let mix_format = *mix_format_ptr;
            let sample_rate = mix_format.nSamplesPerSec;
            let channels = mix_format.nChannels;


            let mut flags = 0;
            if track == AudioTrack::SystemLoopback {
                flags |= AUDCLNT_STREAMFLAGS_LOOPBACK;
            }

            // Initialize stream (0 for buffer duration lets WASAPI choose default)
            let init_result = audio_client
                .Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 0, 0, mix_format_ptr, None);

            // Free the memory allocated by GetMixFormat
            windows::Win32::System::Com::CoTaskMemFree(Some(
                mix_format_ptr as *const core::ffi::c_void,
            ));

            init_result.map_err(|e| {
                CaptureError::Initialization(format!("IAudioClient::Initialize failed: {}", e))
            })?;

            let capture_client: IAudioCaptureClient = audio_client.GetService().map_err(|e| {
                CaptureError::Initialization(format!("Failed to get IAudioCaptureClient: {}", e))
            })?;

            Ok(Self {
                track,
                audio_client,
                capture_client,
                sample_rate,
                channels,
            })
        }
    }
}

impl AudioCapturer for WasapiCapturer {
    fn start(
        &mut self,
        tx: Sender<AudioFrame>,
        stop: Arc<AtomicBool>,
        start_time: Arc<AtomicU64>,
    ) -> Result<(), CaptureError> {
        unsafe {
            // Ensure COM is initialized for the capture thread
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            self.audio_client.Start().map_err(|e| {
                CaptureError::StreamError(format!("Failed to start audio client: {}", e))
            })?;

            while !stop.load(Ordering::Relaxed) {
                // Poll every 10ms
                std::thread::sleep(std::time::Duration::from_millis(10));

                loop {
                    let next_packet_size = self.capture_client.GetNextPacketSize().unwrap_or(0);
                    if next_packet_size == 0 {
                        break;
                    }

                    let mut p_data: *mut u8 = std::ptr::null_mut();
                    let mut num_frames: u32 = 0;
                    let mut flags: u32 = 0;
                    let mut qpc_position: u64 = 0;

                    let hr = self.capture_client.GetBuffer(
                        &mut p_data,
                        &mut num_frames,
                        &mut flags,
                        None,
                        Some(&mut qpc_position),
                    );

                    if hr.is_err() {
                        break;
                    }

                    if num_frames > 0 {
                        let bytes_per_frame = (self.channels * 4) as usize; // assuming f32 (32-bit float)
                        let byte_count = (num_frames as usize) * bytes_per_frame;
                        let slice =
                            std::slice::from_raw_parts(p_data as *const f32, byte_count / 4);

                        let data = slice.to_vec();
                        
                        let qpc_us = qpc_position / 10;
                        let _ = start_time.fetch_min(qpc_us, Ordering::Relaxed);
                        let normalized_ts = qpc_us.saturating_sub(start_time.load(Ordering::Relaxed));

                        let frame = AudioFrame {
                            data,
                            sample_rate: self.sample_rate,
                            channels: self.channels,
                            track: self.track,
                            timestamp_us: normalized_ts,
                        };

                        let _ = tx.send(frame);
                    }

                    let _ = self.capture_client.ReleaseBuffer(num_frames);
                }
            }

            let _ = self.audio_client.Stop();
        }

        Ok(())
    }
}
