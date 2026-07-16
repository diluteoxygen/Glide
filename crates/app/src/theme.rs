//! Central place for every color/spacing constant used in the app.
//! Nothing outside this file should hardcode a `Color32` — import from here
//! so the whole app can be re-themed by editing one place.

use egui::{Color32, Rounding, Stroke};

// ---- Palette -----------------------------------------------------------
// Flat, light, "industry standard" surface. Purple is the ONLY accent color
// in the whole app — used for the selected segment, selected tab, and the
// "on" state of toggle switches. Red is reserved exclusively for the
// record button.

pub const BG_WINDOW: Color32 = Color32::from_rgb(0xF7, 0xF7, 0xF8); // window background
pub const BG_PANEL: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF); // content pane background
pub const BG_CONTROL: Color32 = Color32::from_rgb(0xEC, 0xEC, 0xEE); // unselected pill/segment fill
pub const BG_SIDEBAR_SELECTED: Color32 = Color32::from_rgb(0xE7, 0xE4, 0xF9); // selected tab row fill

pub const ACCENT: Color32 = Color32::from_rgb(0x8B, 0x7C, 0xF6); // purple accent
pub const ACCENT_TEXT: Color32 = Color32::from_rgb(0x4A, 0x3F, 0xA8); // text on light-purple fill

pub const RECORD_RED: Color32 = Color32::from_rgb(0xE8, 0x4A, 0x3F);
pub const RECORD_RED_TEXT: Color32 = Color32::WHITE;

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0x1C, 0x1C, 0x1E);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x6E, 0x6E, 0x76);
pub const TEXT_ON_UNSELECTED_SEGMENT: Color32 = Color32::from_rgb(0x4A, 0x4A, 0x52); // must stay legible — this was a real bug last round

pub const BORDER: Color32 = Color32::from_rgb(0xD9, 0xD9, 0xDD);
pub const DIVIDER: Color32 = Color32::from_rgb(0xE3, 0xE3, 0xE7);

// ---- Shape ---------------------------------------------------------------
// Minimal rounding everywhere — no heavy pill-shaped chrome except the
// Raw/Live segmented control itself, which is a deliberate pill per spec.

pub const RADIUS_SMALL: u8 = 5; // buttons, inputs, dropdown box
pub const RADIUS_ROW: u8 = 6; // sidebar selected-row highlight

pub fn rounding_small() -> egui::CornerRadius {
    egui::CornerRadius::same(RADIUS_SMALL)
}

pub fn rounding_row() -> egui::CornerRadius {
    egui::CornerRadius::same(RADIUS_ROW)
}

pub fn border_stroke() -> Stroke {
    Stroke::new(1.0_f32, BORDER)
}

/// Apply the flat, light base theme to the egui context. Call once at
/// startup (and it's fine to call again if you ever add a theme switch).
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();

    visuals.panel_fill = BG_WINDOW;
    visuals.window_fill = BG_WINDOW;
    visuals.extreme_bg_color = BG_PANEL;

    // No glow/shadow anywhere — flat, professional aesthetic per spec.
    visuals.window_shadow = egui::epaint::Shadow::NONE;
    visuals.popup_shadow = egui::epaint::Shadow::NONE;

    visuals.widgets.noninteractive.bg_fill = BG_CONTROL;
    visuals.widgets.inactive.bg_fill = BG_CONTROL;
    visuals.widgets.hovered.bg_fill = BG_CONTROL.gamma_multiply(0.95);
    visuals.widgets.active.bg_fill = ACCENT;

    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT_TEXT);

    ctx.set_visuals(visuals);

    // Slightly larger default spacing reads calmer/more "professional" than
    // egui's cramped default.
    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    ctx.set_global_style(style);
}
