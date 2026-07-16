//! iOS-style toggle switch, now with the knob sliding via `animate_bool`
//! instead of snapping between the two end positions instantly.

use crate::theme;
use egui::{Color32, CornerRadius, Sense, Ui, Vec2};

/// Draws a toggle bound to `*value`. `id_source` must be unique per
/// toggle instance on screen (e.g. `"audio_record_system"`) — this is
/// required rather than an auto-id because the animation state needs a
/// stable key across frames, and auto-ids inside loops/branches aren't
/// reliably stable in egui.
pub fn show(ui: &mut Ui, id_source: &str, value: &mut bool) -> bool {
    let id = ui.id().with(id_source);
    let size = Vec2::new(38.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    let mut toggled = false;
    if response.clicked() {
        *value = !*value;
        toggled = true;
    }

    let progress = ui.ctx().animate_bool(id, *value);
    if progress > 0.0001 && progress < 0.9999 {
        ui.ctx().request_repaint();
    }

    let track_color = theme::BG_CONTROL.lerp_to_gamma(theme::ACCENT, progress);
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same((size.y / 2.0) as u8), track_color);
    if progress < 0.5 {
        painter.rect_stroke(
            rect,
            CornerRadius::same((size.y / 2.0) as u8),
            egui::Stroke::new(1.0_f32, theme::BORDER),
            egui::StrokeKind::Inside,
        );
    }

    let knob_r = size.y / 2.0 - 3.0;
    let knob_x = rect.left() + size.y / 2.0 + progress * (size.x - size.y);
    painter.circle_filled(egui::pos2(knob_x, rect.center().y), knob_r, Color32::WHITE);

    toggled
}
