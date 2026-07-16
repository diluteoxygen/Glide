# Glide — GUI v3 (full rewrite)

Apple‑language UI for **egui 0.35** (latest, 2026). Every file is complete and
ready to drop into the repo. This doc supersedes the round‑1 and round‑2
handoffs.

---

## What changed (mapped to your complaints)

| Your complaint | What this rewrite does |
| --- | --- |
| **Recording format missing** | First‑class `MKV` / `MP4` selector in the toolbar chip **and** the Recording tab. MKV is the crash‑safe default; MP4 is the final container. |
| **Minimize‑to‑tray toggle ignored** | Fixed at the source. Close now keys off the *actual* setting: ON → hide to tray; **OFF → fully quit**. See `app.rs::handle_close`. |
| **Folder button did nothing** | `reveal` opens the real output folder via the `open` crate. |
| **Mic mute did nothing** | Toolbar mic button toggles `mic_enabled`; muted state shows "system audio only". |
| **Icons selectable by cursor** | Icons are now rasterized **SVG images** (not selectable `RichText`) with a pointer cursor + tooltip. |
| **Submenu buttons were placeholders** | Output picker (`rfd`), launch‑at‑login (`auto-launch`), live disk‑free readout (`sysinfo`), encoder pick, sliders, hotkey capture, reset‑to‑defaults — all wired. |
| **Abrupt bottom cutoff / bare‑bones look** | Real OS chrome, frosted toolbar, grouped macOS settings cards, a persistent status footer (save path + free space), smooth height spring. |

---

## Project layout

```
glide/
├─ Cargo.toml
├─ assets/fonts/            # drop Inter-Regular.ttf + Inter-Medium.ttf here (optional)
└─ src/
   ├─ main.rs
   ├─ app.rs
   ├─ theme.rs
   ├─ config.rs
   ├─ tray.rs
   ├─ icons.rs
   ├─ ui.rs                 # group / row / section helpers
   ├─ widgets/
   │  ├─ mod.rs
   │  ├─ icon_button.rs
   │  ├─ toggle.rs
   │  ├─ segmented.rs
   │  ├─ hotkey.rs
   │  └─ level.rs
   └─ panels/
      ├─ mod.rs
      ├─ recording.rs
      ├─ audio.rs
      ├─ general.rs
      ├─ shortcuts.rs
      └─ advanced.rs
```

---

## Build notes (read first)

1. **egui version**: target `0.35` (the `svg` feature is on by default, which the
   icon system uses). `eframe 0.35` pulls it in.
2. **Fonts (optional)**: put `Inter-Regular.ttf` and `Inter-Medium.ttf` in
   `assets/fonts/`. If absent the app still builds and runs — it just uses egui's
   default font. Loading is defensive (runtime read, not `include_bytes!`).
3. **Icons**: defined inline as SVG strings in `src/icons.rs` — **no asset files
   needed**, nothing to guess. They rasterize through egui's built‑in SVG loader.
4. **Tray**: uses the `tray-icon` crate (which re‑exports `muda`'s `menu`).
   On macOS the app must be a proper `.app` bundle for the tray menu to render
   correctly; in a debug run the icon + left‑click still work.
5. **Dependency versions below are approximations** — run `cargo add` or bump to
   the latest matching line.

> One API to sanity‑check after `cargo add`: `egui::Image::from_bytes(uri, bytes)`
> takes `impl Into<bytes::Bytes>`. Passing a `Vec<u8>` works because
> `bytes::Bytes: From<Vec<u8>>`. If your toolchain complains, wrap with
> `egui::load::Bytes::from(vec)` (and add the `bytes` crate) — nothing else changes.

---

## `Cargo.toml`

```toml
[package]
name = "glide"
version = "0.3.0"
edition = "2021"

[dependencies]
eframe       = "0.35"
egui         = "0.35"
tray-icon    = "0.21"     # brings muda::menu too
image        = { version = "0.25", default-features = false, features = ["png"] }
rfd          = "0.15"     # native folder picker
open         = "5"        # reveal folder in Finder/Explorer
sysinfo      = "0.32"     # free-disk readout
auto-launch  = "0.5"      # launch at login
serde        = { version = "1", features = ["derive"] }
toml         = "0.8"
dirs         = "5"

[profile.release]
opt-level = 2
```

---

## `src/main.rs`

```rust
mod app;
mod config;
mod icons;
mod panels;
mod theme;
mod tray;
mod ui;
mod widgets;

use eframe::egui;

fn main() -> eframe::Result<()> {
    // Load persisted config (or defaults) before the window exists.
    let cfg = config::AppConfig::load();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Glide")
            .with_inner_size([660.0, 138.0])
            .with_min_inner_size([540.0, 138.0])
            // Real OS decorations: the system title bar provides the close
            // button, which is exactly what `minimize-to-tray` intercepts.
            .with_decorations(true)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "glide",
        options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);       // fonts + visuals, once
            let tray = tray::init(&cc.egui_ctx); // keep alive for app lifetime
            Ok(Box::new(app::GlideApp::new(cfg, tray)))
        }),
    )
}
```

---

## `src/theme.rs`

```rust
//! Apple / macOS-System-Settings inspired tokens + visuals.
//! Edit colors here to re-theme the whole app.

use egui::{
    epaint::Shadow, Color32, FontData, FontDefinitions, FontFamily, FontId, Margin, Rounding,
    Stroke, TextStyle, Vec2, Visuals,
};

// ---- palette ----
pub const WINDOW: Color32 = Color32::from_rgb(0xF6, 0xF6, 0xF7);
pub const FIELD: Color32 = Color32::from_rgb(0xEC, 0xEC, 0xED);
pub const CARD: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const CONTROL: Color32 = Color32::from_rgb(0xE9, 0xE9, 0xEB);

pub const ACCENT: Color32 = Color32::from_rgb(0x0A, 0x84, 0xFF);
pub const ACCENT_PRESS: Color32 = Color32::from_rgb(0x00, 0x60, 0xDF);
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(0x0A, 0x84, 0xFF, 0x26);

pub const RED: Color32 = Color32::from_rgb(0xFF, 0x3B, 0x30);
pub const RED_SOFT: Color32 = Color32::from_rgba_premultiplied(0xFF, 0x3B, 0x30, 0x26);
pub const GREEN: Color32 = Color32::from_rgb(0x34, 0xC7, 0x59);
pub const ORANGE: Color32 = Color32::from_rgb(0xFF, 0x9F, 0x0A);

pub const TEXT1: Color32 = Color32::from_rgb(0x1D, 0x1D, 0x1F);
pub const TEXT2: Color32 = Color32::from_rgb(0x6E, 0x6E, 0x73);
pub const TEXT3: Color32 = Color32::from_rgb(0x9B, 0x9B, 0xA1);

/// Hairline separator (~12% black). Premultiplied so it composites cleanly.
pub const HAIR: Color32 = Color32::from_rgba_premultiplied(60, 60, 67, 31);

pub const R_CARD: f32 = 11.0;

pub fn rounding_card() -> Rounding {
    Rounding::same(R_CARD)
}
pub fn hair_stroke() -> Stroke {
    Stroke::new(1.0, HAIR)
}

/// Per-channel color lerp (no dependency on a possibly-renamed egui helper).
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);

    let mut v = Visuals::light();
    v.panel_fill = WINDOW;
    v.window_fill = WINDOW;
    v.extreme_bg_color = CARD;
    v.faint_bg_color = Color32::TRANSPARENT;
    v.window_shadow = Shadow::NONE;
    v.popup_shadow = Shadow::NONE;
    v.widgets.noninteractive.bg_fill = CONTROL;
    v.widgets.noninteractive.bg_stroke = Stroke::NONE;
    v.widgets.inactive.bg_fill = CONTROL;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.bg_fill = Color32::from_rgb(0xDF, 0xDF, 0xE2);
    v.widgets.hovered.bg_stroke = Stroke::NONE;
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.bg_stroke = Stroke::NONE;
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::NONE;
    v.override_text_color = Some(TEXT1);
    ctx.set_visuals(v);

    let mut s = (*ctx.style()).clone();
    s.spacing.item_spacing = Vec2::new(8.0, 6.0);
    s.spacing.button_padding = Vec2::new(10.0, 5.0);
    s.spacing.indent = 16.0;
    s.text_styles = [
        (TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
    ]
    .into();
    ctx.set_style(s);
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let regular = std::fs::read("assets/fonts/Inter-Regular.ttf").ok();
    let medium = std::fs::read("assets/fonts/Inter-Medium.ttf").ok();
    if let (Some(reg), Some(med)) = (regular, medium) {
        fonts.font_data.insert("Inter-Regular".to_owned(), FontData::from_owned(reg));
        fonts.font_data.insert(
            "Inter-Medium".to_owned(),
            FontData::from_owned(med).tweak(egui::FontTweak {
                y_offset_factor: 0.0,
                y_offset: 0.0,
                scale: 1.0,
                base_font: false,
            }),
        );
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
            fam.insert(0, "Inter-Regular".to_owned());
            fam.insert(1, "Inter-Medium".to_owned());
        }
    } else {
        eprintln!("[glide] Inter fonts not found in assets/fonts — using default font.");
    }
    ctx.set_fonts(fonts);
}
```

---

## `src/icons.rs`

```rust
//! Crisp, themeable icons rendered as inline SVG -> rasterized by egui's
//! built-in SVG loader. No icon-font crate, no glyph-name guessing, no asset
//! files. Every icon is drawn in white and tinted at the call site, so the
//! whole set recolors with the theme.
//!
//! Requires egui's default `svg` feature (on by default in eframe).

use egui::{Color32, Image, Response, Ui, Vec2};

/// Returns the SVG source for a named icon.
pub fn svg(name: &str) -> &'static str {
    match name {
        "record" => S_RECORD,
        "stop" => S_STOP,
        "mic" => S_MIC,
        "micOff" => S_MIC_OFF,
        "folder" => S_FOLDER,
        "reveal" => S_REVEAL,
        "settings" => S_SETTINGS,
        "chevronDown" => S_CHEVRON,
        "waveform" => S_WAVE,
        "keyboard" => S_KBD,
        "film" => S_FILM,
        "gauge" => S_GAUGE,
        "drive" => S_DRIVE,
        "check" => S_CHECK,
        "info" => S_INFO,
        "cursor" => S_CURSOR,
        "clock" => S_CLOCK,
        "sparkles" => S_SPARKLES,
        _ => S_INFO,
    }
}

/// Build a tinted, exactly-sized image widget for the given icon.
pub fn image(name: &str, size: f32, color: Color32) -> Image<'static> {
    Image::from_bytes(format!("embedded://{name}.svg"), svg(name).as_bytes().to_vec())
        .fit_to_exact_size(Vec2::splat(size))
        .tint(color)
}

/// Add an icon inline.
pub fn draw(ui: &mut Ui, name: &str, size: f32, color: Color32) -> Response {
    ui.add(image(name, size, color))
}

macro_rules! icon {
    ($body:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">"#,
            $body,
            "</svg>"
        )
    };
}
macro_rules! fill_icon {
    ($body:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="#fff" stroke="none">"#,
            $body,
            "</svg>"
        )
    };
}

const S_RECORD: &str = fill_icon!(r#"<circle cx="12" cy="12" r="6.5"/>"#);
const S_STOP: &str = fill_icon!(r#"<rect x="6.5" y="6.5" width="11" height="11" rx="2.4"/>"#);
const S_MIC: &str = icon!(r#"<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11.5a6.5 6.5 0 0 0 13 0"/><path d="M12 18v3M9 21h6"/>"#);
const S_MIC_OFF: &str = icon!(r#"<path d="M9 5.5a3 3 0 0 1 6 .5v3"/><path d="M15.5 11.5A3.5 3.5 0 0 1 9 12.2"/><path d="M5.5 11.5a6.5 6.5 0 0 0 10.8 4.8"/><path d="M12 18v3M9 21h6"/><path d="M4 4l16 16"/>"#);
const S_FOLDER: &str = icon!(r#"<path d="M3 7.5a2 2 0 0 1 2-2h3.4a2 2 0 0 1 1.4.6l1 1a2 2 0 0 0 1.4.6H19a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7.5Z"/>"#);
const S_REVEAL: &str = icon!(r#"<path d="M3 7.5a2 2 0 0 1 2-2h4l2 2H19a2 2 0 0 1 2 2v1"/><path d="M3.5 11H21l-1.4 5.2a2 2 0 0 1-1.9 1.5H8a2 2 0 0 1-1.9-1.4L3.5 11Z"/><path d="M14 16l4-4M18 16v-4h-4"/>"#);
const S_SETTINGS: &str = icon!(r#"<circle cx="12" cy="12" r="3.2"/><path d="M12 2.5v2.6M12 18.9v2.6M21.5 12h-2.6M5.1 12H2.5M18.7 5.3l-1.8 1.8M7.1 16.9l-1.8 1.8M18.7 18.7l-1.8-1.8M7.1 7.1 5.3 5.3"/>"#);
const S_CHEVRON: &str = icon!(r#"<path d="M6 9.5l6 6 6-6"/>"#);
const S_WAVE: &str = icon!(r#"<path d="M4 9v6M8 5v14M12 7v10M16 9.5v5M20 7.5v9"/>"#);
const S_KBD: &str = icon!(r#"<rect x="2.5" y="6" width="19" height="12" rx="2.5"/><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M9 14h6"/>"#);
const S_FILM: &str = icon!(r#"<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M8 4v16M16 4v16M3 9h5M3 15h5M16 9h5M16 15h5"/>"#);
const S_GAUGE: &str = icon!(r#"<path d="M4 18a8 8 0 1 1 16 0"/><path d="M12 14l4-3.5"/>"#);
const S_DRIVE: &str = icon!(r#"<path d="M4 14h16l-1.8-7.2A2 2 0 0 0 16.2 5H7.8a2 2 0 0 0-1.95 1.55L4 14Z"/><path d="M4 14h16a1.5 1.5 0 0 1 1.5 1.5V18a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 18v-2.5A1.5 1.5 0 0 1 4 14Z"/><path d="M7.5 17h.01"/>"#);
const S_CHECK: &str = icon!(r#"<path d="M5 12.5l4.5 4.5L19 7.5"/>"#);
const S_INFO: &str = icon!(r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h.01"/>"#);
const S_CURSOR: &str = icon!(r#"<path d="M5 3.5l6.5 16 2.2-6.6 6.6-2.2L5 3.5Z"/>"#);
const S_CLOCK: &str = icon!(r#"<circle cx="12" cy="12" r="9"/><path d="M12 7.5V12l3 2"/>"#);
const S_SPARKLES: &str = icon!(r#"<path d="M12 4l1.6 4.4L18 10l-4.4 1.6L12 16l-1.6-4.4L6 10l4.4-1.6L12 4Z"/>"#);
```

---

## `src/widgets/mod.rs`

```rust
pub mod hotkey;
pub mod icon_button;
pub mod level;
pub mod segmented;
pub mod toggle;

pub use icon_button::IconButton;
```

---

## `src/widgets/icon_button.rs`

```rust
//! Toolbar icon button: pointer cursor, hover/active background, tooltip, and
//! crucially an *image* glyph (not selectable text) — fixes the
//! "users can select the icons" bug.

use crate::icons;
use crate::theme;
use egui::{CursorIcon, Response, Sense, Stroke, Ui, Vec2, Widget};

pub struct IconButton {
    pub icon: &'static str,
    pub size: f32,
    pub active: bool,
    pub danger: bool,
    pub tooltip: Option<&'static str>,
}

impl Default for IconButton {
    fn default() -> Self {
        Self { icon: "info", size: 18.0, active: false, danger: false, tooltip: None }
    }
}

impl Widget for IconButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let dim = 34.0;
        let (rect, mut resp) = ui.allocate_exact_size(Vec2::splat(dim), Sense::click());
        let p = ui.painter();

        let (bg, stroke, fg) = if self.active {
            let base = if self.danger { theme::RED_SOFT } else { theme::ACCENT_SOFT };
            let col = if self.danger { theme::RED } else { theme::ACCENT };
            (base, Stroke::new(0.5, col), col)
        } else if resp.hovered() {
            (theme::CONTROL.gamma_multiply(0.8), Stroke::new(0.5, theme::HAIR), theme::TEXT1)
        } else {
            (egui::Color32::TRANSPARENT, Stroke::NONE, theme::TEXT1)
        };

        let inner = rect.shrink(1.0);
        if bg != egui::Color32::TRANSPARENT {
            p.rect_filled(inner, egui::Rounding::same(9.0), bg);
        }
        if stroke != Stroke::NONE {
            p.rect_stroke(inner, egui::Rounding::same(9.0), stroke);
        }

        // image glyph centered (not selectable text)
        let _ = ui.put(
            egui::Rect::from_center_size(rect.center(), Vec2::splat(self.size)),
            icons::image(self.icon, self.size, fg),
        );

        resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        if let Some(t) = self.tooltip {
            resp = resp.on_hover_text(t);
        }
        resp
    }
}
```

---

## `src/widgets/toggle.rs`

```rust
//! iOS/macOS toggle with a sliding knob (animate_bool).

use crate::theme;
use egui::{Color32, Sense, Ui, Vec2};

const OFF_TRACK: Color32 = Color32::from_rgba_premultiplied(120, 120, 128, 82);

/// Returns true the frame the user toggled it.
pub fn show(ui: &mut Ui, id: &str, value: &mut bool) -> bool {
    let size = Vec2::new(38.0, 22.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

    let changed = if resp.clicked() {
        *value = !*value;
        true
    } else {
        false
    };

    let t = ui.ctx().animate_bool(ui.id().with(id), *value);
    if t > 0.001 && t < 0.999 {
        ui.ctx().request_repaint();
    }

    let p = ui.painter();
    p.rect_filled(rect, egui::Rounding::same(size.y / 2.0), theme::lerp_color(OFF_TRACK, theme::ACCENT, t));
    let knob_r = size.y / 2.0 - 3.0;
    let kx = rect.left() + size.y / 2.0 + t * (size.x - size.y);
    p.circle_filled(egui::pos2(kx, rect.center().y + 0.6), knob_r, Color32::BLACK.linear_multiply(0.06));
    p.circle_filled(egui::pos2(kx, rect.center().y), knob_r, theme::CARD);

    changed
}
```

---

## `src/widgets/segmented.rs`

```rust
//! Equal-width segmented control with a sliding highlight.
//! Returns Some(i) when the user picks a different segment.

use crate::theme;
use egui::{CursorIcon, FontId, Pos2, Rect, Rounding, Sense, Stroke, Ui};

pub fn show(ui: &mut Ui, id: &str, labels: &[&str], current: usize) -> Option<usize> {
    let pad = 2.0;
    let h = 26.0;
    let seg_w = 58.0;
    let n = labels.len().max(1);
    let total = egui::Vec2::new(seg_w * n as f32 + pad * 2.0, h + pad * 2.0);

    let (rect, _resp) = ui.allocate_exact_size(total, Sense::hover());
    let p = ui.painter();

    p.rect_filled(rect, Rounding::same(total.y / 2.0), theme::CONTROL);

    // smoothed animated index
    let aid = ui.id().with(id);
    let dt = ui.input(|i| i.raw.delta_time).clamp(0.001, 0.05);
    let target = current as f32;
    let cur = ui.ctx().data(|d| d.get_temp::<f32>(aid)).unwrap_or(target);
    let next = cur + (target - cur) * (1.0 - (-16.0 * dt).exp());
    ui.ctx().data_mut(|d| d.insert_temp(aid, next));
    if (next - target).abs() > 0.002 {
        ui.ctx().request_repaint();
    }

    // sliding white thumb
    let hl = Rect::from_min_size(
        Pos2::new(rect.left() + pad + next * seg_w, rect.top() + pad),
        egui::Vec2::new(seg_w, h),
    );
    p.rect_filled(hl, Rounding::same(h / 2.0), theme::CARD);
    p.rect_stroke(hl, Rounding::same(h / 2.0), Stroke::new(0.5, theme::HAIR));

    let mut picked = None;
    for (i, label) in labels.iter().enumerate() {
        let sr = Rect::from_min_size(
            Pos2::new(rect.left() + pad + i as f32 * seg_w, rect.top() + pad),
            egui::Vec2::new(seg_w, h),
        );
        let r = ui.interact(sr, ui.id().with((id, i)), Sense::click());
        let sel = i == current;
        let color = if sel { theme::TEXT1 } else { theme::TEXT2 };
        p.text(sr.center(), egui::Align2::CENTER_CENTER, label, FontId::proportional(12.5), color);
        if r.clicked() {
            picked = Some(i);
        }
        r.on_hover_cursor(CursorIcon::PointingHand);
    }
    picked
}
```

---

## `src/widgets/hotkey.rs`

```rust
//! Click "Edit" to listen, then press a combo to capture it.

use crate::theme;
use egui::{Event, Key, Modifiers, RichText, Ui};

pub enum HotkeyAction {
    None,
    ToggleListen,
    Captured(String),
}

pub fn show(ui: &mut Ui, label: &str, value: &str, listening: bool) -> HotkeyAction {
    let mut action = HotkeyAction::None;

    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0).color(theme::TEXT1));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let btn = if listening {
                ui.add(egui::Button::new("Press keys…").fill(theme::ACCENT).stroke(egui::Stroke::NONE))
            } else {
                ui.button("Edit")
            };
            if btn.clicked() {
                action = HotkeyAction::ToggleListen;
            }

            let (r, _) = ui.allocate_exact_size(egui::vec2(110.0, 26.0), egui::Sense::hover());
            let p = ui.painter();
            p.rect_filled(r, egui::Rounding::same(6.0), theme::CONTROL);
            p.rect_stroke(r, egui::Rounding::same(6.0), theme::hair_stroke());
            let text = if listening { "Listening…" } else { value };
            p.text(r.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::monospace(12.0), theme::TEXT1);
        });
    });

    if listening {
        if let Some(combo) = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                Event::Key { key, pressed: true, modifiers, .. } => Some(format_combo(*modifiers, *key)),
                _ => None,
            })
        }) {
            action = HotkeyAction::Captured(combo);
        }
    }
    action
}

fn format_combo(m: Modifiers, key: Key) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if m.command { parts.push("⌘"); }
    if m.ctrl { parts.push("⌃"); }
    if m.alt { parts.push("⌥"); }
    if m.shift { parts.push("⇧"); }
    parts.push(&format!("{:?}", key));
    parts.join("")
}
```

---

## `src/widgets/level.rs`

```rust
//! Animated microphone input meter.

use crate::theme;
use egui::{Pos2, Rect, Sense, Ui, Vec2};

pub fn show(ui: &mut Ui, active: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(150.0, 22.0), Sense::hover());
    let n = 20usize;
    let gap = 2.0;
    let bw = (rect.width() / n as f32 - gap).max(2.0);
    let id = ui.id().with("lvl");

    let t = ui.input(|i| i.time) as f32;
    let target = if active { 0.25 + 0.65 * ((t * 6.0).sin() * 0.5 + 0.5).min(1.0) } else { 0.05 };
    let cur = ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(0.05);
    let next = cur + (target - cur) * 0.25;
    ui.ctx().data_mut(|d| d.insert_temp(id, next));
    if active {
        ui.ctx().request_repaint();
    }

    for i in 0..n {
        let frac = i as f32 / n as f32;
        let on = frac < next;
        let col = if frac > 0.8 { theme::RED } else if frac > 0.6 { theme::ORANGE } else { theme::GREEN };
        let h = (frac * 0.9 + 0.1) * rect.height();
        let x = rect.left() + i as f32 * (bw + gap);
        let r = Rect::from_min_size(Pos2::new(x, rect.bottom() - h), Vec2::new(bw, h));
        ui.painter().rect_filled(r, egui::Rounding::same(1.0), if on { col } else { theme::CONTROL });
    }
}
```

---

## `src/ui.rs`

```rust
//! Shared layout helpers for the macOS-style grouped settings.

use crate::theme;
use egui::{Align, Layout, Margin, RichText, Ui};

pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).strong().color(theme::TEXT3));
    ui.add_space(4.0);
}

pub fn group(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(theme::CARD)
        .stroke(theme::hair_stroke())
        .rounding(theme::rounding_card())
        .inner_margin(Margin { left: 14, right: 14, top: 4, bottom: 4 })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });
}

/// A row with a title (+optional subtitle) on the left and a control on the right.
pub fn row(ui: &mut Ui, title: &str, subtitle: Option<&str>, right: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(13.0).color(theme::TEXT1));
            if let Some(s) = subtitle {
                ui.label(RichText::new(s).size(11.0).color(theme::TEXT2));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), right);
    });
}
```

---

## `src/config.rs`

```rust
//! Persisted settings. `#[serde(default)]` makes the file forward-compatible.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Mode { Raw, Live }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Format { Mkv, Mp4 }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Encoder {
    Auto,
    #[serde(rename = "VideoToolbox")]
    VideoToolbox,
    Nvenc,
    Qsv,
    Amf,
    Vaapi,
    X264,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub mode: Mode,
    pub format: Format,
    pub export_mp4_copy: bool,

    pub resolution: String,
    pub framerate: u32,
    pub quality: String,
    pub capture_cursor: bool,
    pub capture_clicks: bool,
    pub hardware_accel: bool,

    pub mic_source: String,
    pub mic_enabled: bool,
    pub system_audio: bool,

    pub output_path: String,
    pub show_hud: bool,
    pub launch_at_login: bool,
    pub minimize_to_tray: bool, // <-- the toggle the bug was about
    pub countdown: u32,
    pub post_summary: bool,

    pub hotkey_start_stop: String,
    pub hotkey_pause_resume: String,

    pub encoder: Encoder,
    pub ring_buffer: u32,
    pub zoom_intensity: u32,
    pub verbose_logging: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let out = dirs::video_dir().or_else(dirs::home_dir).unwrap_or_default().join("Glide");
        Self {
            mode: Mode::Live,
            format: Format::Mkv,
            export_mp4_copy: true,
            resolution: "1080p".into(),
            framerate: 60,
            quality: "Crisp".into(),
            capture_cursor: true,
            capture_clicks: true,
            hardware_accel: true,
            mic_source: "MacBook Pro Microphone".into(),
            mic_enabled: true,
            system_audio: true,
            output_path: out.to_string_lossy().into_owned(),
            show_hud: true,
            launch_at_login: false,
            minimize_to_tray: true,
            countdown: 3,
            post_summary: true,
            hotkey_start_stop: "⌃⇧R".into(),
            hotkey_pause_resume: "⌃⇧P".into(),
            encoder: Encoder::Auto,
            ring_buffer: 4,
            zoom_intensity: 65,
            verbose_logging: false,
        }
    }
}

impl AppConfig {
    pub fn path() -> std::path::PathBuf {
        dirs::config_dir().unwrap_or_default().join("glide").join("config.toml")
    }
    pub fn load() -> Self {
        match std::fs::read_to_string(Self::path()) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }
    pub fn save(&self) {
        if let Some(parent) = Self::path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(s) = toml::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), s);
        }
    }
}
```

---

_Continued in part 2 — `tray.rs`, `panels/*`, and `app.rs` (the close-to-tray fix)._
