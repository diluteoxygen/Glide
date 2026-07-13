use capture_core::{AudioFrame, AudioTrack};
use crossbeam_channel::bounded;
use encode::audio::AudioEncoder;

fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    ffmpeg_next::init().unwrap();

    let mut encoder = AudioEncoder::new(48000, 2).unwrap();
    let (tx, _rx) = bounded(10000);

    // Simulate 20 seconds of audio at 48kHz
    // 48000 samples per second. Chunks of 960 samples (20ms) per channel.
    let samples_per_chunk = 960;
    let floats_per_chunk = samples_per_chunk * 2;
    
    let mut current_us: u64 = 0;
    
    for i in 0..1000 {
        let frame = AudioFrame {
            data: vec![0.0f32; floats_per_chunk],
            sample_rate: 48000,
            channels: 2,
            track: AudioTrack::SystemLoopback,
            timestamp_us: current_us,
        };
        
        encoder.encode(frame, 0, &tx).unwrap();
        
        current_us += 20_000; // 20ms advance
        
        if i == 500 {
            // Simulate 100ms gap
            current_us += 100_000;
            tracing::info!("--- SIMULATED GAP OF 100ms ---");
        }
    }
}
