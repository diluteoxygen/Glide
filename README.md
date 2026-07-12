# Glide

Glide is a lightweight, high-performance, cross-platform (Windows and Linux) screen recorder written in Rust.

It is designed to be highly efficient by leveraging native capture APIs and hardware-accelerated encoding, offering an extremely low-overhead raw recording mode.

In addition to standard raw recording, Glide features a post-processing "Demo Mode". This mode logs cursor and input events during capture and applies a spring-damped virtual camera render pass. The result is a polished product-demo video with smooth panning and automatic cursor-following zoom, similar to tools like Screen Studio or CleanShot.

## Features

- **High-Performance Capture:** Uses DXGI Desktop Duplication on Windows and PipeWire on Linux for zero-copy, GPU-resident frame capture.
- **Hardware Encoding:** Defaults to hardware encoders (NVENC, QSV, AMF, VAAPI) with a software x264 fallback only if necessary.
- **Crash-Resilient:** Writes directly to MKV during recording to ensure the file is safe and playable even in the event of a system crash.
- **Demo Mode:** Post-process raw footage into cinematic, cursor-following product demos.

## Building from source

Ensure you have the latest stable Rust toolchain installed.

### Windows
```sh
cargo build --release
```

### Linux
You will need the PipeWire development headers:
```sh
sudo apt-get install libpipewire-0.3-dev
cargo build --release
```
