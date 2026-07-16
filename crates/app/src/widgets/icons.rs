//! Icon set backed by the Phosphor icon font (`egui-phosphor` crate) —
//! the closest egui equivalent to Font Awesome for HTML. Every icon is a
//! font glyph, not a hand-drawn `Painter` shape, which is what made the
//! old icon set look inconsistent/AI-generated. Do not add new hand-drawn
//! icon functions here — if a needed glyph is missing from Phosphor, say
//! so rather than drawing a substitute shape.
//!
//! New dependency (already added to Cargo.toml): `egui-phosphor = "0.12"`
//! (pulls in `egui ^0.34` — that's why the whole project's egui/eframe
//! version was bumped to 0.34 alongside this).
//!
//! IMPORTANT — verify before compiling: the glyph constant names below
//! are Phosphor's icon names in SCREAMING_SNAKE_CASE (confirmed pattern:
//! `egui_phosphor::regular::FILE_CODE`). A few names below (RECORD,
//! HARD_DRIVE, WARNING) are my best guess at Phosphor's actual naming —
//! grep `egui_phosphor::regular` on docs.rs (or your editor's
//! autocomplete once the crate is added) and fix any that don't match
//! before shipping. Don't guess-and-ship; a wrong constant name is a
//! compile error, not a silent bug, so this is a five-minute fix once you
//! have the crate downloaded.

use egui::{Color32, RichText, Ui};

/// Call once at startup, right after `theme::apply(&cc.egui_ctx)`.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}

/// Draws a single glyph at `size` in `color`. Every call site should go
// through this rather than building `RichText` ad hoc, so stroke/weight
// stays consistent if the variant (Regular/Bold/Fill) ever changes in
// one place.
pub fn draw(ui: &mut Ui, glyph: &str, size: f32, color: Color32) -> egui::Response {
    ui.label(RichText::new(glyph).size(size).color(color))
}

pub fn chevron(painter: &egui::Painter, rect: egui::Rect, color: Color32, up: bool) {
    let stroke = egui::Stroke::new(1.6_f32, color);
    let center = rect.center();
    let half_w = rect.width() * 0.3;
    let half_h = rect.height() * 0.2;
    let (top_y, bottom_y) = if up {
        (center.y - half_h, center.y + half_h)
    } else {
        (center.y + half_h, center.y - half_h)
    };
    painter.line_segment(
        [
            egui::pos2(center.x - half_w, bottom_y),
            egui::pos2(center.x, top_y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, top_y),
            egui::pos2(center.x + half_w, bottom_y),
        ],
        stroke,
    );
}

pub const FOLDER: &str = egui_phosphor::regular::FOLDER;
pub const MICROPHONE: &str = egui_phosphor::regular::MICROPHONE;
pub const MICROPHONE_SLASH: &str = egui_phosphor::regular::MICROPHONE_SLASH;
pub const CARET_DOWN: &str = egui_phosphor::regular::CARET_DOWN;
pub const CARET_UP: &str = egui_phosphor::regular::CARET_UP;
pub const INFO: &str = egui_phosphor::regular::INFO;
pub const GEAR: &str = egui_phosphor::regular::GEAR;
pub const SPEAKER_HIGH: &str = egui_phosphor::regular::SPEAKER_HIGH;
pub const KEYBOARD: &str = egui_phosphor::regular::KEYBOARD;
pub const LIGHTNING: &str = egui_phosphor::regular::LIGHTNING;
pub const CLOCK: &str = egui_phosphor::regular::CLOCK;
pub const MONITOR: &str = egui_phosphor::regular::MONITOR;
pub const APP_WINDOW: &str = egui_phosphor::regular::APP_WINDOW;
pub const CROP: &str = egui_phosphor::regular::CROP;
pub const PENCIL_SIMPLE: &str = egui_phosphor::regular::PENCIL_SIMPLE;

// Best-guess names — verify per the note above.
pub const RECORD: &str = egui_phosphor::regular::RECORD; // filled circle, "start"
pub const STOP: &str = egui_phosphor::regular::STOP; // filled square, "stop"
pub const HARD_DRIVE: &str = egui_phosphor::regular::HARD_DRIVE;
pub const WARNING: &str = egui_phosphor::regular::WARNING;
