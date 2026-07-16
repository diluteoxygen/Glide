# (part 2 — append to glide-source.md)

---

## `src/tray.rs`

```rust
//! System-tray icon + menu. Left-click (or "Show Glide") re-opens the window;
//! "Quit Glide" forces a real exit even when minimize-to-tray is on.

use egui::{Context, ViewportCommand};
use image::{ImageBuffer, Rgba};
use std::sync::{atomic::AtomicBool, OnceLock};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

static CTX: OnceLock<Context> = OnceLock::new();
struct Ids {
    show: MenuId,
    record: MenuId,
    quit: MenuId,
}
static IDS: OnceLock<Ids> = OnceLock::new();

/// Set true by the tray "Quit" menu so the close handler lets the app exit.
pub static FORCE_QUIT: AtomicBool = AtomicBool::new(false);
/// Set true by the tray "Start/Stop" menu; the app drains & toggles recording.
pub static PENDING_RECORD: AtomicBool = AtomicBool::new(false);

pub fn init(ctx: &Context) -> TrayIcon {
    let _ = CTX.set(ctx.clone());

    let menu = Menu::new();
    let show = MenuItem::new("Show Glide", true, None);
    let record = MenuItem::new("Start / Stop recording", true, None);
    let quit = MenuItem::new("Quit Glide", true, None);
    let _ = IDS.set(Ids {
        show: show.id().clone(),
        record: record.id().clone(),
        quit: quit.id().clone(),
    });
    let _ = menu.append(&show);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&record);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit);

    TrayIconBuilder::new()
        .with_tooltip("Glide")
        .with_icon(make_icon())
        .with_menu(Box::new(menu))
        .build()
        .expect("failed to build tray icon")
}

/// Call each frame from `update` to react to tray interactions.
pub fn pump(ctx: &Context) {
    // Menu selections
    while let Ok(ev) = MenuEvent::receiver().try_recv() {
        if let Some(ids) = IDS.get() {
            if &ev.id == &ids.show {
                show_window();
            } else if &ev.id == &ids.record {
                PENDING_RECORD.store(true, std::sync::atomic::Ordering::SeqCst);
                ctx.request_repaint();
            } else if &ev.id == &ids.quit {
                FORCE_QUIT.store(true, std::sync::atomic::Ordering::SeqCst);
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }
    // Icon clicks (left-click reopens)
    while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
        match ev {
            TrayIconEvent::Click { button: tray_icon::MouseButton::Left, button_state, .. }
                if !button_state.is_pressed() =>
            {
                show_window();
            }
            TrayIconEvent::DoubleClick { button: tray_icon::MouseButton::Left, .. } => {
                show_window();
            }
            _ => {}
        }
    }
}

fn show_window() {
    if let Some(ctx) = CTX.get() {
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

/// 32x32 app-style icon drawn procedurally (no asset file needed).
fn make_icon() -> Icon {
    let s = 32u32;
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(s, s);
    let r = 9.0f32;
    let cx = (s / 2) as f32;
    let cy = (s / 2) as f32;
    for y in 0..s {
        for x in 0..s {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let inside = rounded_rect_contains(px, py, s as f32, s as f32, r);
            if !inside {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                continue;
            }
            // diagonal blue->indigo gradient
            let t = (x + y) as f32 / (2 * s) as f32;
            let cr = (10.0 + t * 100.0) as u8;
            let cg = (132.0 - t * 40.0) as u8;
            let cb = (255.0 - t * 5.0) as u8;
            // white aperture dot in the center
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
            if d < 5.0 {
                img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            } else if d < 8.5 {
                img.put_pixel(x, y, Rgba([cr, cg, cb, 255]));
            } else {
                img.put_pixel(x, y, Rgba([cr, cg, cb, 255]));
            }
        }
    }
    Icon::from_rgba(img.into_raw(), s, s).expect("bad icon rgba")
}

fn rounded_rect_contains(x: f32, y: f32, w: f32, h: f32, r: f32) -> bool {
    // distance to nearest rounded-rect edge corner
    let dx = (x - r).max(0.0).min(w - r);
    let dy = (y - r).max(0.0).min(h - r);
    (x - dx) * (x - dx) + (y - dy) * (y - dy) <= r * r
}
```

---

## `src/panels/mod.rs`

```rust
pub mod advanced;
pub mod audio;
pub mod general;
pub mod recording;
pub mod shortcuts;

use crate::config::AppConfig;
use crate::panels::shortcuts::HotkeyField;
use egui::Ui;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tab {
    Recording,
    Audio,
    General,
    Shortcuts,
    Advanced,
}

pub const TABS: [Tab; 5] = [
    Tab::Recording,
    Tab::Audio,
    Tab::General,
    Tab::Shortcuts,
    Tab::Advanced,
];
pub const TAB_LABELS: [&str; 5] = ["Recording", "Audio", "General", "Shortcuts", "Advanced"];
pub const TAB_ICONS: [&str; 5] = ["film", "waveform", "settings", "keyboard", "gauge"];
pub const TAB_BLURB: [&str; 5] = [
    "Format, resolution & quality",
    "Microphone & system sound",
    "Output, tray & launch",
    "Global hotkeys & gestures",
    "Encoder & pipeline",
];

pub fn show(ui: &mut Ui, tab: Tab, cfg: &mut AppConfig, listening: &mut Option<HotkeyField>) {
    match tab {
        Tab::Recording => recording::show(ui, cfg),
        Tab::Audio => audio::show(ui, cfg),
        Tab::General => general::show(ui, cfg),
        Tab::Shortcuts => shortcuts::show(ui, cfg, listening),
        Tab::Advanced => advanced::show(ui, cfg),
    }
}
```

---

## `src/panels/recording.rs`

```rust
use crate::config::{AppConfig, Format};
use crate::theme;
use crate::ui::{group, row, section_label};
use crate::widgets::{segmented, toggle};
use egui::{ComboBox, Ui};

pub fn show(ui: &mut Ui, cfg: &mut AppConfig) {
    section_label(ui, "Container");
    group(ui, |ui| {
        let cur = if cfg.format == Format::Mkv { 0 } else { 1 };
        row(ui, "Recording format", Some(if cfg.format == Format::Mkv {
            "MKV is crash-safe — a power loss keeps the file playable."
        } else {
            "MP4 is universally compatible, used for the final export."
        }), |ui| {
            if let Some(i) = segmented::show(ui, "fmt", &["MKV", "MP4"], cur) {
                cfg.format = if i == 0 { Format::Mkv } else { Format::Mp4 };
            }
        });
        ui.separator();
        row(ui, "Export an MP4 copy after recording", Some("Records MKV first, then remuxes a tidy MP4 when you stop."), |ui| {
            toggle::show(ui, "export_mp4", &mut cfg.export_mp4_copy);
        });
    });

    ui.add_space(14.0);
    section_label(ui, "Quality");
    group(ui, |ui| {
        row(ui, "Resolution", None, |ui| {
            ComboBox::from_id_salt("res")
                .selected_text(&cfg.resolution)
                .show_ui(ui, |ui| {
                    for r in ["720p", "1080p", "1440p", "4K"] {
                        ui.selectable_value(&mut cfg.resolution, r.to_string(), r);
                    }
                });
        });
        ui.separator();
        row(ui, "Frame rate", None, |ui| {
            let cur = if cfg.framerate >= 60 { 1 } else { 0 };
            if let Some(i) = segmented::show(ui, "fps", &["30 fps", "60 fps"], cur) {
                cfg.framerate = if i == 0 { 30 } else { 60 };
            }
        });
        ui.separator();
        row(ui, "Quality", None, |ui| {
            ComboBox::from_id_salt("qual")
                .selected_text(format!("{} quality", cfg.quality))
                .show_ui(ui, |ui| {
                    for q in ["Smooth", "Balanced", "Crisp", "Max"] {
                        ui.selectable_value(&mut cfg.quality, q.to_string(), q);
                    }
                });
        });
    });

    ui.add_space(14.0);
    section_label(ui, "Capture");
    group(ui, |ui| {
        row(ui, "Capture the cursor", Some("Include the pointer in the recording."), |ui| {
            toggle::show(ui, "cursor", &mut cfg.capture_cursor);
        });
        ui.separator();
        row(ui, "Highlight cursor clicks", Some("Render a soft ripple on each click."), |ui| {
            toggle::show(ui, "clicks", &mut cfg.capture_clicks);
        });
        ui.separator();
        row(ui, "Hardware acceleration", Some("Use VideoToolbox / NVENC when available for lower CPU."), |ui| {
            toggle::show(ui, "hwaccel", &mut cfg.hardware_accel);
        });
    });

    let _ = theme::TEXT1; // keep import used
}
```

---

## `src/panels/audio.rs`

```rust
use crate::config::AppConfig;
use crate::ui::{group, row, section_label};
use crate::widgets::{level, toggle};
use crate::{icons, theme};
use egui::{ComboBox, Ui};

pub fn show(ui: &mut Ui, cfg: &mut AppConfig) {
    section_label(ui, "Microphone");
    group(ui, |ui| {
        row(ui, "Input source", None, |ui| {
            ComboBox::from_id_salt("micsrc")
                .selected_text(&cfg.mic_source)
                .show_ui(ui, |ui| {
                    for d in ["MacBook Pro Microphone", "AirPods Pro", "Blue Yeti USB", "System Default"] {
                        ui.selectable_value(&mut cfg.mic_source, d.to_string(), d);
                    }
                });
        });
        ui.separator();
        row(ui, "Input level", None, |ui| {
            level::show(ui, cfg.mic_enabled);
        });
        ui.separator();
        row(ui, "Record microphone", Some("Off = system audio only."), |ui| {
            toggle::show(ui, "mic", &mut cfg.mic_enabled);
        });
    });

    ui.add_space(14.0);
    section_label(ui, "System");
    group(ui, |ui| {
        row(ui, "Record system audio", Some("Capture everything playing on this Mac via loopback."), |ui| {
            toggle::show(ui, "sys", &mut cfg.system_audio);
        });
    });

    let _ = (icons::image, theme::TEXT1);
}
```

---

## `src/panels/general.rs`

```rust
use crate::config::AppConfig;
use crate::theme;
use crate::ui::{group, row, section_label};
use crate::widgets::{segmented, toggle};
use crate::{icons, ui as ghui};
use egui::{RichText, Ui};

pub fn show(ui: &mut Ui, cfg: &mut AppConfig) {
    section_label(ui, "Output");
    group(ui, |ui| {
        ui.horizontal(|ui| {
            icons::draw(ui, "folder", 18.0, theme::TEXT2);
            ui.vertical(|ui| {
                ui.label(RichText::new("Save recordings to").size(13.0).color(theme::TEXT1));
                ui.label(RichText::new(&cfg.output_path).size(11.5).color(theme::TEXT2));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Change…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().set_directory(&cfg.output_path).pick_folder() {
                        cfg.output_path = p.to_string_lossy().into_owned();
                        cfg.save();
                    }
                }
                if ui.button("Reveal").clicked() {
                    let _ = std::fs::create_dir_all(&cfg.output_path);
                    let _ = open::that(&cfg.output_path);
                }
            });
        });
    });

    ui.add_space(14.0);
    section_label(ui, "Behavior");
    group(ui, |ui| {
        row(ui, "Show recording HUD", Some("A floating overlay with elapsed time & zoom level — never burned into the video."), |ui| {
            toggle::show(ui, "hud", &mut cfg.show_hud);
        });
        ui.separator();
        row(ui, "Launch Glide at login", None, |ui| {
            if toggle::show(ui, "login", &mut cfg.launch_at_login) {
                set_login(cfg.launch_at_login);
                cfg.save();
            }
        });
        ui.separator();
        row(
            ui,
            "Minimize to tray when closed",
            Some(if cfg.minimize_to_tray {
                "ON — closing hides Glide to the menu bar instead of quitting."
            } else {
                "OFF — closing fully quits Glide. It will not hide to the tray."
            }),
            |ui| {
                if toggle::show(ui, "tray", &mut cfg.minimize_to_tray) {
                    cfg.save();
                }
            },
        );
        ui.separator();
        let cur = match cfg.countdown { 0 => 0, 3 => 1, _ => 2 };
        row(ui, "Countdown before recording", None, |ui| {
            if let Some(i) = segmented::show(ui, "count", &["Off", "3s", "5s"], cur) {
                cfg.countdown = [0, 3, 5][i];
            }
        });
    });

    ui.add_space(14.0);
    section_label(ui, "After recording");
    group(ui, |ui| {
        row(ui, "Show post-recording summary", Some("File size, duration and a “Show in folder” button when you stop."), |ui| {
            toggle::show(ui, "summary", &mut cfg.post_summary);
        });
    });

    let _ = ghui::body; // keep helper import live
}

fn set_login(enable: bool) {
    let exe = std::env::current_exe().ok();
    let Ok(exe) = exe else { return };
    let al = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Glide")
        .set_app_path(&exe.to_string_lossy())
        .build();
    let res = if enable { al.enable() } else { al.disable() };
    if let Err(e) = res {
        eprintln!("[glide] launch-at-login: {e}");
    }
}
```

---

## `src/panels/shortcuts.rs`

```rust
use crate::config::AppConfig;
use crate::ui::{group, section_label};
use crate::widgets::hotkey::{self, HotkeyAction};
use egui::{RichText, Ui};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HotkeyField {
    StartStop,
    PauseResume,
}

pub fn show(ui: &mut Ui, cfg: &mut AppConfig, listening: &mut Option<HotkeyField>) {
    section_label(ui, "Global hotkeys");
    group(ui, |ui| {
        let lis_start = *listening == Some(HotkeyField::StartStop);
        match hotkey::show(ui, "Start / Stop recording", &cfg.hotkey_start_stop, lis_start) {
            HotkeyAction::ToggleListen => {
                *listening = if lis_start { None } else { Some(HotkeyField::StartStop) }
            }
            HotkeyAction::Captured(c) => {
                cfg.hotkey_start_stop = c;
                *listening = None;
                cfg.save();
            }
            _ => {}
        }
        ui.separator();
        let lis_pause = *listening == Some(HotkeyField::PauseResume);
        match hotkey::show(ui, "Pause / Resume", &cfg.hotkey_pause_resume, lis_pause) {
            HotkeyAction::ToggleListen => {
                *listening = if lis_pause { None } else { Some(HotkeyField::PauseResume) }
            }
            HotkeyAction::Captured(c) => {
                cfg.hotkey_pause_resume = c;
                *listening = None;
                cfg.save();
            }
            _ => {}
        }
    });

    ui.add_space(14.0);
    section_label(ui, "Live-zoom gestures · fixed");
    ui.label(
        RichText::new("Part of the gesture engine — can't be remapped, always active in Live mode.")
            .size(11.5)
            .color(crate::theme::TEXT2),
    );
    ui.add_space(4.0);
    group(ui, |ui| {
        gesture(ui, "⇧⇧", "Double-tap Shift", "Engage follow / reset to default zoom");
        ui.separator();
        gesture(ui, "⌃⌃", "Double-tap Control", "Return to 1.0×");
        ui.separator();
        gesture(ui, "⇧ + Scroll", "Shift + Scroll", "Adjust zoom level directly");
    });
}

fn gesture(ui: &mut Ui, combo: &str, name: &str, effect: &str) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(
            RichText::new(combo).font(egui::FontId::monospace(11.5)).color(crate::theme::TEXT1),
        ));
        ui.label(RichText::new(name).size(13.0).color(crate::theme::TEXT1));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(effect).size(11.5).color(crate::theme::TEXT2));
        });
    });
}
```

---

## `src/panels/advanced.rs`

```rust
use crate::config::{AppConfig, Encoder};
use crate::theme;
use crate::ui::{group, row, section_label};
use crate::widgets::toggle;
use egui::{ComboBox, RichText, Slider, Ui};

pub fn show(ui: &mut Ui, cfg: &mut AppConfig) {
    section_label(ui, "Encoder");
    group(ui, |ui| {
        row(ui, "Video encoder", None, |ui| {
            ComboBox::from_id_salt("enc")
                .selected_text(enc_label(cfg.encoder))
                .show_ui(ui, |ui| {
                    for e in [
                        Encoder::Auto,
                        Encoder::VideoToolbox,
                        Encoder::Nvenc,
                        Encoder::Qsv,
                        Encoder::Amf,
                        Encoder::Vaapi,
                        Encoder::X264,
                    ] {
                        ui.selectable_value(&mut cfg.encoder, e, enc_label(e));
                    }
                });
        });
        ui.separator();
        row(ui, "Detected", None, |ui| {
            ui.label(RichText::new("VideoToolbox (H.265) · probed at launch").size(12.0).color(theme::TEXT2));
        });
    });

    ui.add_space(14.0);
    section_label(ui, "Pipeline");
    group(ui, |ui| {
        row(ui, "Zoom intensity", Some("Mapped to the camera solver's spring damping."), |ui| {
            ui.add(Slider::new(&mut cfg.zoom_intensity, 0..=100).clamp_to_range(true).text("%"));
        });
        ui.separator();
        row(ui, "Ring buffer", Some("Frames buffered before the encoder; higher = smoother under load."), |ui| {
            ui.add(Slider::new(&mut cfg.ring_buffer, 1..=16).clamp_to_range(true).text("fr"));
        });
    });

    ui.add_space(14.0);
    section_label(ui, "Diagnostics");
    group(ui, |ui| {
        row(ui, "Verbose logging", Some("Write detailed pipeline traces to the log file."), |ui| {
            toggle::show(ui, "verbose", &mut cfg.verbose_logging);
        });
    });

    ui.add_space(12.0);
    if ui.button("Reset Advanced to defaults").clicked() {
        cfg.encoder = Encoder::Auto;
        cfg.ring_buffer = 4;
        cfg.zoom_intensity = 65;
        cfg.verbose_logging = false;
        cfg.save();
    }
}

fn enc_label(e: Encoder) -> &'static str {
    match e {
        Encoder::Auto => "Auto (recommended)",
        Encoder::VideoToolbox => "VideoToolbox · Apple GPU",
        Encoder::Nvenc => "NVENC · NVIDIA",
        Encoder::Qsv => "Quick Sync · Intel",
        Encoder::Amf => "AMF · AMD",
        Encoder::Vaapi => "VAAPI · Linux",
        Encoder::X264 => "Software · x264",
    }
}
```

---

## `src/app.rs`

```rust
//! Main window: toolbar, status footer, animated settings panel, and the
//! fixed close→tray logic.

use crate::config::{AppConfig, Format, Mode};
use crate::icons;
use crate::panels::{self, Tab};
use crate::panels::shortcuts::HotkeyField;
use crate::theme;
use crate::ui::{group, row, section_label};
use crate::widgets::{segmented, toggle, IconButton};
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Layout, Margin, Pos2, Rect, Rounding, Sense,
    Stroke, Vec2, ViewportCommand,
};
use std::time::{Duration, Instant};
use tray_icon::TrayIcon;

const COLLAPSED: f32 = 150.0;
const EXPANDED: f32 = 420.0;
const DEFAULT_W: f32 = 660.0;

pub struct GlideApp {
    cfg: AppConfig,
    tray: TrayIcon,
    tab: Tab,
    settings_open: bool,
    listening: Option<HotkeyField>,

    is_recording: bool,
    started_at: Option<Instant>,

    last_sent: Option<Vec2>,

    // cached free-space readout (time, free_gb, est_minutes)
    disk: Option<(Instant, f64, u64)>,
    toast: Option<(Instant, String, Color32)>,
}

impl GlideApp {
    pub fn new(cfg: AppConfig, tray: TrayIcon) -> Self {
        Self {
            cfg,
            tray,
            tab: Tab::Recording,
            settings_open: false,
            listening: None,
            is_recording: false,
            started_at: None,
            last_sent: None,
            disk: None,
            toast: None,
        }
    }

    // ── THE headline fix ────────────────────────────────────────────────
    // When the OS close button is pressed, egui sets `close_requested`. The
    // root viewport would then exit — UNLESS we send `CancelClose`. We gate
    // that on the *actual* setting value, which is what round-2 got wrong
    // (it always hid to tray regardless of the toggle).
    fn handle_close(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if !close_requested {
            return;
        }

        if crate::tray::FORCE_QUIT.swap(false, std::sync::atomic::Ordering::SeqCst) {
            // Tray "Quit" → let the app exit. Do nothing.
            return;
        }

        if self.cfg.minimize_to_tray {
            // Setting ON → hide to tray instead of quitting.
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
            self.toast("Hid to the menu bar", theme::GREEN);
        }
        // else: setting OFF → do nothing → Glide fully quits. (The fix.)
    }

    fn toast(&mut self, msg: &str, color: Color32) {
        self.toast = Some((Instant::now(), msg.to_string(), color));
    }

    fn refresh_disk(&mut self) {
        let needs = match self.disk {
            None => true,
            Some((t, _, _)) => t.elapsed() > Duration::from_secs(5),
        };
        if !needs {
            return;
        }
        use sysinfo::Disks;
        let disks = Disks::new_with_refreshed_list();
        // pick the disk whose mount point is a prefix of the output path (else first)
        let target = std::path::Path::new(&self.cfg.output_path);
        let pick = disks
            .list()
            .iter()
            .find(|d| target.starts_with(d.mount_point()))
            .or_else(|| disks.list().first());
        if let Some(d) = pick {
            let free_b = d.available_space();
            let free_gb = free_b as f64 / 1_073_741_824.0;
            // ~6 Mbps → 0.75 MB/s → minutes
            let mins = (free_b as f64 / (0.75 * 1_048_576.0)) / 60.0;
            self.disk = Some((Instant::now(), free_gb, mins as u64));
        }
    }

    fn apply_sizing(&mut self, ctx: &egui::Context, target_h: f32) {
        // Don't fight the OS while maximized (the round-1 maximize bug).
        let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
        if maximized {
            self.last_sent = None;
            return;
        }
        let w = ctx
            .input(|i| i.viewport().outer_rect.map(|r| r.width()))
            .unwrap_or(DEFAULT_W);
        let desired = Vec2::new(w, target_h);
        let changed = match self.last_sent {
            Some(last) => (last - desired).length() > 0.5,
            None => true,
        };
        if changed {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(desired));
            self.last_sent = Some(desired);
        }
    }

    fn toggle_record(&mut self) {
        if self.is_recording {
            self.is_recording = false;
            let secs = self.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);
            self.started_at = None;
            let ext = if self.cfg.export_mp4_copy { "mp4" } else { self.cfg.format.as_ext() };
            if self.cfg.post_summary {
                self.toast(
                    &format!("Saved · {}s · .{}", secs, ext),
                    theme::GREEN,
                );
            }
            // BACKEND HOOK: stop the pipeline; if export_mp4_copy, run the remux.
        } else {
            self.is_recording = true;
            self.started_at = Some(Instant::now());
            if self.settings_open {
                self.settings_open = false;
            }
            if self.cfg.countdown > 0 {
                self.toast(&format!("Recording in {}s…", self.cfg.countdown), theme::ACCENT);
            }
            // BACKEND HOOK: start the pipeline in Raw or Live mode.
        }
    }
}

impl Format {
    fn as_ext(self) -> &'static str {
        match self {
            Format::Mkv => "mkv",
            Format::Mp4 => "mp4",
        }
    }
}

impl eframe::App for GlideApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_close(ctx); // <-- minimize-to-tray fix lives here
        crate::tray::pump(ctx);

        // pending record toggle from the tray menu
        if crate::tray::PENDING_RECORD.swap(false, std::sync::atomic::Ordering::SeqCst) {
            self.toggle_record();
        }

        if self.is_recording {
            ctx.request_repaint_after(Duration::from_millis(250));
        }

        // expand/collapse spring
        let progress = ctx.animate_bool(egui::Id::new("expand"), self.settings_open);
        if progress > 0.001 && progress < 0.999 {
            ctx.request_repaint();
        }
        let target_h = COLLAPSED + EXPANDED * progress;
        self.apply_sizing(ctx, target_h);
        self.refresh_disk();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::WINDOW).inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                self.toolbar(ui);
                ui.add_space(10.0);
                self.status_row(ui);

                if progress > 0.01 {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    let avail = EXPANDED * progress;
                    ui.scope(|ui| {
                        ui.set_max_height(avail.max(0.0));
                        ui.set_clip_rect(ui.max_rect());
                        self.settings(ui, progress);
                    });
                }
            });

        self.draw_toast(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&egui::Context>) {
        self.cfg.save();
    }
}

// ── layout pieces ──────────────────────────────────────────────────────
impl GlideApp {
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Mode
            let cur = if self.cfg.mode == Mode::Live { 1 } else { 0 };
            if let Some(i) = segmented::show(ui, "mode", &["Raw", "Live"], cur) {
                self.cfg.mode = if i == 0 { Mode::Raw } else { Mode::Live };
            }

            ui.add_space(6.0);
            self.format_chip(ui);

            ui.separator();

            // Mic
            if ui
                .add(
                    IconButton {
                        icon: if self.cfg.mic_enabled { "mic" } else { "micOff" },
                        active: !self.cfg.mic_enabled,
                        danger: true,
                        tooltip: Some(if self.cfg.mic_enabled {
                            "Mute microphone — record system audio only"
                        } else {
                            "Unmute microphone"
                        }),
                        ..Default::default()
                    },
                )
                .clicked()
            {
                self.cfg.mic_enabled = !self.cfg.mic_enabled;
                self.toast(
                    if self.cfg.mic_enabled { "Microphone on" } else { "Muted — system audio only" },
                    theme::TEXT1,
                );
            }

            // Reveal folder
            if ui
                .add(IconButton {
                    icon: "reveal",
                    tooltip: Some("Reveal output folder"),
                    ..Default::default()
                })
                .clicked()
            {
                let _ = std::fs::create_dir_all(&self.cfg.output_path);
                let _ = open::that(&self.cfg.output_path);
                self.toast("Revealed in Finder", theme::ACCENT);
            }

            // Settings
            if ui
                .add(IconButton {
                    icon: "settings",
                    active: self.settings_open,
                    tooltip: Some("Settings"),
                    ..Default::default()
                })
                .clicked()
            {
                self.settings_open = !self.settings_open;
            }

            // Right side
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.record_button(ui);
                if self.is_recording {
                    ui.add_space(10.0);
                    self.timer(ui);
                    ui.add_space(8.0);
                    self.pulse(ui);
                }
            });
        });
    }

    fn format_chip(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(78.0, 28.0), Sense::click());
        let p = ui.painter();
        p.rect_filled(rect, Rounding::same(8.0), theme::CARD);
        p.rect_stroke(rect, Rounding::same(8.0), theme::hair_stroke());

        let (label, color) = match self.cfg.format {
            Format::Mkv => ("MKV", theme::ACCENT),
            Format::Mp4 => ("MP4", theme::ORANGE),
        };
        p.circle_filled(Pos2::new(rect.left() + 13.0, rect.center().y), 3.5, color);
        p.text(
            Pos2::new(rect.left() + 24.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(12.0),
            theme::TEXT1,
        );

        resp.on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text("Recording format — click to switch. MKV is crash-safe; MP4 is for the final export.");
        if resp.clicked() {
            self.cfg.format = if self.cfg.format == Format::Mkv { Format::Mp4 } else { Format::Mkv };
        }
    }

    fn record_button(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(132.0, 38.0), Sense::click());
        let p = ui.painter();

        let (fill, fg, glyph, label) = if self.is_recording {
            (Color32::from_rgb(0x1C, 0x1C, 0x1E), theme::RED, "stop", "Stop")
        } else {
            (theme::RED, Color32::WHITE, "record", "Record")
        };
        p.rect_filled(rect, Rounding::same(19.0), fill);
        // icon
        let _ = ui.put(
            Rect::from_center_size(Pos2::new(rect.left() + 22.0, rect.center().y), Vec2::splat(14.0)),
            icons::image(glyph, 14.0, fg),
        );
        p.text(
            Pos2::new(rect.left() + 40.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.5),
            fg,
        );

        resp.on_hover_cursor(CursorIcon::PointingHand);
        if resp.clicked() {
            self.toggle_record();
        }
    }

    fn timer(&self, ui: &mut egui::Ui) {
        let secs = self.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        let text = if h > 0 {
            format!("{}:{:02}:{:02}", h, m, s)
        } else {
            format!("{:02}:{:02}", m, s)
        };
        ui.label(egui::RichText::new(text).monospace().size(13.0).color(theme::TEXT2));
    }

    fn pulse(&self, ui: &mut egui::Ui) {
        let t = ui.input(|i| i.time) as f32;
        let a = 0.4 + 0.6 * ((t * 3.0).sin() * 0.5 + 0.5);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 4.0, theme::RED.gamma_multiply(a));
        ui.ctx().request_repaint();
    }

    fn status_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            icons::draw(ui, "drive", 14.0, theme::TEXT3);
            ui.label(
                egui::RichText::new(format!("Saved to {}", self.cfg.output_path))
                    .size(11.0)
                    .color(theme::TEXT2),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some((_, gb, mins)) = self.disk {
                    ui.label(
                        egui::RichText::new(format!("{:.0} GB free · ~{} min remaining", gb, mins))
                            .size(11.0)
                            .color(theme::TEXT2),
                    );
                }
                ui.label(
                    egui::RichText::new(match self.cfg.mode {
                        Mode::Raw => "Raw capture · uncompressed feed",
                        Mode::Live => "Live zoom · double-tap ⇧ to follow the cursor",
                    })
                    .size(11.0)
                    .color(theme::TEXT3),
                );
            });
        });
    }

    fn settings(&mut self, ui: &mut egui::Ui, _progress: f32) {
        ui.horizontal_top(|ui| {
            // Sidebar
            ui.vertical(|ui| {
                ui.set_width(168.0);
                ui.set_min_height(EXPANDED - 24.0);
                for (i, tab) in panels::TABS.iter().enumerate() {
                    let selected = *tab == self.tab;
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
                    if selected {
                        ui.painter().rect_filled(rect, Rounding::same(8.0), theme::ACCENT);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, Rounding::same(8.0), theme::CONTROL);
                    }
                    let col = if selected { Color32::WHITE } else { theme::TEXT2 };
                    let _ = ui.put(
                        Rect::from_center_size(Pos2::new(rect.left() + 18.0, rect.center().y), Vec2::splat(15.0)),
                        icons::image(panels::TAB_ICONS[i], 15.0, col),
                    );
                    ui.painter().text(
                        Pos2::new(rect.left() + 34.0, rect.center().y - 6.0),
                        Align2::LEFT_CENTER,
                        panels::TAB_LABELS[i],
                        FontId::proportional(12.5),
                        if selected { Color32::WHITE } else { theme::TEXT1 },
                    );
                    ui.painter().text(
                        Pos2::new(rect.left() + 34.0, rect.center().y + 7.0),
                        Align2::LEFT_CENTER,
                        panels::TAB_BLURB[i],
                        FontId::proportional(9.5),
                        if selected { Color32::from_rgba_premultiplied(255,255,255,160) } else { theme::TEXT3 },
                    );
                    if resp.clicked() {
                        self.tab = *tab;
                    }
                    resp.on_hover_cursor(CursorIcon::PointingHand);
                }
            });

            ui.separator();

            // Content (scrollable field area with footer)
            ui.vertical(|ui| {
                ui.push_id("content", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(EXPANDED - 64.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            egui::Frame::none().fill(theme::FIELD).inner_margin(Margin::same(12.0)).show(ui, |ui| {
                                panels::show(ui, self.tab, &mut self.cfg, &mut self.listening);
                            });
                        });
                });
            });
        });
    }

    fn draw_toast(&self, ctx: &egui::Context) {
        if let Some((t, msg, color)) = &self.toast {
            if t.elapsed() > Duration::from_millis(2600) {
                return;
            }
            let screen = ctx.screen_rect();
            egui::Area::new(egui::Id::new("toast"))
                .order(egui::Order::Foreground)
                .fixed_pos(Pos2::new(screen.center().x - 150.0, screen.bottom() - 56.0))
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(28, 28, 32))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.set_width(300.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("●").color(*color).size(12.0));
                                ui.label(egui::RichText::new(msg).color(Color32::WHITE).size(12.5));
                            });
                        });
                });
        }
    }
}

// silence unused imports kept for the grouped helpers used by panels
#[allow(unused_imports)]
use crate::ui as _u;
```

---

## How the fix maps to the code

- **`app.rs::handle_close`** is the single source of truth for "what does the
  close button do". It reads `cfg.minimize_to_tray` (the toggle the user set in
  General) and branches:
  - tray‑menu Quit / setting **OFF** → real exit (nothing sent).
  - setting **ON** → `CancelClose` + `Visible(false)`.
- **`tray.rs`** owns the menu; `pump()` runs each frame. "Show Glide" / icon
  click re‑show the hidden window via `ViewportCommand::Visible(true)` + `Focus`.
- **Folder** button (`app.rs::toolbar`) calls `open::that(output_path)`.
- **Mic** button toggles `cfg.mic_enabled` (muted ⇒ system audio only).
- **Format** chip cycles MKV/MP4 live; the Recording tab has the full selector.
- **Icons** are SVG images via `icons::image(...)` → not selectable, pointer
  cursor + tooltip (`IconButton`).

That's the entire app. `cargo run` and iterate.
