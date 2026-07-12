use std::env;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("Starting Glide CLI (Phase 0 scaffolding)");
    info!("Platform: {}", env::consts::OS);

    // Cheap GPU vendor check
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
        let info = adapter.get_info();
        info!("Detected GPU: {} ({:?})", &info.name, info.backend);
    }
}
