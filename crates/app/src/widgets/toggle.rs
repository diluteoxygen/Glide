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
