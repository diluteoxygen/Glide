# Architecture

## Overview

Two pipelines share a common capture/encode/mux core:

```
Video capture thread ─┐
                       ├─> Ring buffer ─> Encode thread ─> Mux thread ─> .mkv
Audio capture thread ─┘
```

Demo mode adds a second pass that runs after recording stops:

```
Raw .mkv + cursor/event log ─> Camera path solver ─> GPU render pass ─> Encode + mux ─> final .mp4/.mkv
```

The recording pipeline and the render pipeline are deliberately separate
processes/passes. Recording must stay simple and low-latency; the camera
render pass is allowed to be slower than real time.

## Stage 1: Capture

Two platform backends behind one trait:

```rust
pub trait VideoCapturer {
    fn start(&mut self, tx: Sender<Frame>, stop: Arc<AtomicBool>) -> Result<(), CaptureError>;
}
```

- **Windows**: DXGI Desktop Duplication API for the core GPU-side capture
  loop. Layer Windows.Graphics.Capture (WGC) on top when per-window capture,
  cursor toggling, or HDR is needed — WGC is what OBS has moved to for those
  features. Capture is GPU-resident until the final copy to a staging
  texture for CPU-visible mapping.
- **Linux**: PipeWire via `xdg-desktop-portal`'s ScreenCast interface. This
  is the only path that works uniformly under Wayland and (via XWayland)
  X11. Keep an X11 XShm fallback behind a feature flag for environments
  without a working portal (older DEs, minimal window managers).

Audio mirrors this structure:

- **Windows**: WASAPI loopback capture for system audio, WASAPI input for
  mic, mixed or kept as separate tracks (decide in Phase 2).
- **Linux**: PipeWire audio streams (PulseAudio compatibility layer covers
  older systems automatically).

Frames carry an OS-provided presentation timestamp, not a
locally-generated one — this is what keeps A/V sync correct even if a
thread stalls briefly.

## Stage 2: Ring buffer

`crossbeam-channel` bounded channel, one per media type (video, audio).
Capture threads use `try_send` — if the encoder is behind, the newest frame
is dropped rather than blocking the GPU-side capture. Document the chosen
channel capacity and the reasoning in this file once Phase 1/3 settles on a
number (start with something small, e.g. 3-4 frames, and tune based on
observed encode latency).

## Stage 3: Encode

`ffmpeg-next` bindings to `libavcodec`/`libavformat`. Encoder selection
order, tried at startup and cached:

1. Windows + NVIDIA: `h264_nvenc` / `hevc_nvenc`
2. Windows/Linux + Intel: `h264_qsv`
3. Windows + AMD: `h264_amf`
4. Linux (Intel/AMD): `h264_vaapi`
5. Fallback: `libx264` (software)

Encode thread owns the encoder context, receives frames from the ring
buffer, and pushes encoded packets to the mux thread via a second channel.

## Stage 4: Mux

Writes to MKV as recording happens (not buffered to the end) so a crash or
kill leaves a playable file. Remux to MP4 is a separate, explicit export
step once recording is confirmed complete and healthy.

## Stage 5 (demo mode only): Cursor & event log

Sampled independently of video frame rate — 60-120Hz cursor position,
plus discrete click/scroll/key events, plus active window bounds when
available. Stored as a simple timestamped log (e.g. a flat binary or
JSONL file alongside the raw recording) keyed to the same clock as the
video presentation timestamps.

## Stage 6 (demo mode only): Camera path solver

Each of camera x, y, and zoom is modeled as an independent **second-order
dynamical system** (critically damped spring, Muratori-style), not a
keyframe/lerp timeline. Parameters:

- Position (x, y): responsive, `f≈1.2-1.8Hz`, `z≈0.9-1.0`, slight negative
  response for anticipatory motion.
- Zoom: slower and heavier, `f≈0.5-0.8Hz`, `z≈1.0`, no anticipation.

Zoom target logic: zoom in (~1.5-1.8x) toward the cursor/click cluster on
sustained activity, ease back to 1.0x after ~1.5s of idle. Feed the spring a
debounced/smoothed cursor target, not raw mouse jitter, or the jitter leaks
through. A small lookahead buffer (200-400ms) lets zoom-on-click feel
proactive instead of reactive — this is the concrete reason this stage
cannot run live during capture.

Output per output-frame: `(center_x, center_y, zoom)`.

## Stage 7 (demo mode only): GPU render pass

`wgpu`-based compositor. For each output frame: decode the raw frame,
compute the crop rect from the camera solver's `(center_x, center_y, zoom)`
(size `frame_w/zoom × frame_h/zoom`, clamped to stay inside frame bounds),
sample that region into the output canvas via a textured quad + bilinear
sampling fragment shader. This is a UV remap, not ML — cheap and
predictable in cost.

Rendered frames feed back into Stage 3/4 (encode + mux) to produce the
final output file.

## Threading model summary

One OS thread per stage (video capture, audio capture, encode, mux; plus,
in the render pass, decode, camera solve, GPU render, re-encode, mux).
Stages communicate exclusively via bounded channels. No stage holds a lock
another stage waits on.

## Decisions

_(Claude Code: append dated entries here as implementation decisions are
made that aren't already specified above — ring buffer capacity, exact
encoder probing logic, event log file format, etc. Keep each entry to a
few sentences: what was decided and why.)_

- **2026-07-12**: For the Linux capture handshake, we are using `ashpd` instead of raw `zbus` to avoid hand-rolled D-Bus boilerplate.
- **2026-07-12**: For Phase 1, we are using raw `BGRA` as the pixel format mapping from DXGI and PipeWire to keep the initial capture MVP simple and verifiable. We will defer the optimization to `NV12` (planar YUV) to a later phase once the basic pipeline and encoders are integrated.
- **2026-07-12**: In the DXGI capture loop, we explicitly drop/skip frames where `DXGI_OUTDUPL_FRAME_INFO.LastPresentTime == 0`. Desktop Duplication yields these frames when the hardware cursor moves but the actual desktop image hasn't composited a new frame. Skipping them prevents severe artificial framerate inflation and unnecessary GPU-to-CPU memory copies when the desktop is idle.
- **2026-07-12**: For Phase 2 (Audio), we decided to capture system loopback audio and microphone audio into **separate** tracks rather than mixing them. This preserves flexibility for post-processing features (like muting the mic or applying noise gates) and aligns with the eventual multi-track MKV muxing.
- **2026-07-12**: For Phase 3 (Encode/Mux), we are using `libswresample` to actively resample all incoming audio streams to a unified 48kHz before encoding to AAC. This guarantees consistent MKV compatibility even if the system loopback and microphone operate at different native sample rates.
- **2026-07-12**: For Phase 3 (Encode/Mux), out-of-order muxing from the three asynchronous encode threads (1 video, 2 audio) is handled automatically by FFmpeg's `av_interleaved_write_frame` (`write_interleaved` in `ffmpeg-next`). This delegates the `dts`-based packet buffering and reordering to libavformat, avoiding a complex hand-rolled priority queue in Rust.
- **2026-07-12**: For the FFmpeg dependency on Windows, we decided to use `vcpkg` (`ffmpeg:x64-windows`) in manifest mode instead of auto-downloading BtbN binaries. MSVC-built headers avoid the bindgen/MinGW ABI mismatch. We use `vcpkg.json` to pin the baseline to a commit providing FFmpeg 6.1.1, because FFmpeg 7's opaque structs break `ffmpeg-sys-next` 6.1.0's hardcoded bindgen layout tests on MSVC.
- **2026-07-12**: For MSVC FFmpeg bindings on Windows, we forked `ffmpeg-sys-next` 6.1.0 into the workspace to commit pre-generated `bindings_x86_64-pc-windows-msvc.rs` directly to the repo. This entirely removes the requirement for developers (and CI) to have LLVM/Clang installed locally. The bindings are generated behind a `generate-bindings` feature flag that bumps to `bindgen 0.72.1` (to fix an opacity bug in 0.64) while strictly preserving the original `ParseCallbacks` hook (which coerces macros like `AV_CODEC_FLAG_INTERLACED_ME` to `u32` to avoid `E0308` type mismatches in `ffmpeg-next`).
- **2026-07-12**: (Phase 3 Encode Note) The `ffmpeg-next` API has removed `Context::new_with_codec`. When resuming Phase 3 encoding implementation, update `crates/encode` to use the modern context initialization flow (e.g. `Context::new` or `Context::from_parameters`).
