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
                baseline_offset_factor: 0.0,
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
