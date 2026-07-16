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

macro_rules! make_icon {
    ($body:literal) => {
        concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"#fff\" stroke-width=\"1.7\" stroke-linecap=\"round\" stroke-linejoin=\"round\">",
            $body,
            "</svg>"
        )
    };
}
macro_rules! make_fill_icon {
    ($body:literal) => {
        concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"#fff\" stroke=\"none\">",
            $body,
            "</svg>"
        )
    };
}

const S_RECORD: &str = make_fill_icon!("<circle cx=\"12\" cy=\"12\" r=\"6.5\"/>");
const S_STOP: &str = make_fill_icon!("<rect x=\"6.5\" y=\"6.5\" width=\"11\" height=\"11\" rx=\"2.4\"/>");
const S_MIC: &str = make_icon!("<rect x=\"9\" y=\"3\" width=\"6\" height=\"11\" rx=\"3\"/><path d=\"M5.5 11.5a6.5 6.5 0 0 0 13 0\"/><path d=\"M12 18v3M9 21h6\"/>");
const S_MIC_OFF: &str = make_icon!("<path d=\"M9 5.5a3 3 0 0 1 6 .5v3\"/><path d=\"M15.5 11.5A3.5 3.5 0 0 1 9 12.2\"/><path d=\"M5.5 11.5a6.5 6.5 0 0 0 10.8 4.8\"/><path d=\"M12 18v3M9 21h6\"/><path d=\"M4 4l16 16\"/>");
const S_FOLDER: &str = make_icon!("<path d=\"M3 7.5a2 2 0 0 1 2-2h3.4a2 2 0 0 1 1.4.6l1 1a2 2 0 0 0 1.4.6H19a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7.5Z\"/>");
const S_REVEAL: &str = make_icon!("<path d=\"M3 7.5a2 2 0 0 1 2-2h4l2 2H19a2 2 0 0 1 2 2v1\"/><path d=\"M3.5 11H21l-1.4 5.2a2 2 0 0 1-1.9 1.5H8a2 2 0 0 1-1.9-1.4L3.5 11Z\"/><path d=\"M14 16l4-4M18 16v-4h-4\"/>");
const S_SETTINGS: &str = make_icon!("<circle cx=\"12\" cy=\"12\" r=\"3.2\"/><path d=\"M12 2.5v2.6M12 18.9v2.6M21.5 12h-2.6M5.1 12H2.5M18.7 5.3l-1.8 1.8M7.1 16.9l-1.8 1.8M18.7 18.7l-1.8-1.8M7.1 7.1 5.3 5.3\"/>");
const S_CHEVRON: &str = make_icon!("<path d=\"M6 9.5l6 6 6-6\"/>");
const S_WAVE: &str = make_icon!("<path d=\"M4 9v6M8 5v14M12 7v10M16 9.5v5M20 7.5v9\"/>");
const S_KBD: &str = make_icon!("<rect x=\"2.5\" y=\"6\" width=\"19\" height=\"12\" rx=\"2.5\"/><path d=\"M6 10h.01M10 10h.01M14 10h.01M18 10h.01M9 14h6\"/>");
const S_FILM: &str = make_icon!("<rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2.5\"/><path d=\"M8 4v16M16 4v16M3 9h5M3 15h5M16 9h5M16 15h5\"/>");
const S_GAUGE: &str = make_icon!("<path d=\"M4 18a8 8 0 1 1 16 0\"/><path d=\"M12 14l4-3.5\"/>");
const S_DRIVE: &str = make_icon!("<path d=\"M4 14h16l-1.8-7.2A2 2 0 0 0 16.2 5H7.8a2 2 0 0 0-1.95 1.55L4 14Z\"/><path d=\"M4 14h16a1.5 1.5 0 0 1 1.5 1.5V18a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 18v-2.5A1.5 1.5 0 0 1 4 14Z\"/><path d=\"M7.5 17h.01\"/>");
const S_CHECK: &str = make_icon!("<path d=\"M5 12.5l4.5 4.5L19 7.5\"/>");
const S_INFO: &str = make_icon!("<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M12 11v5M12 8h.01\"/>");
const S_CURSOR: &str = make_icon!("<path d=\"M5 3.5l6.5 16 2.2-6.6 6.6-2.2L5 3.5Z\"/>");
const S_CLOCK: &str = make_icon!("<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M12 7.5V12l3 2\"/>");
const S_SPARKLES: &str = make_icon!("<path d=\"M12 4l1.6 4.4L18 10l-4.4 1.6L12 16l-1.6-4.4L6 10l4.4-1.6L12 4Z\"/>");
