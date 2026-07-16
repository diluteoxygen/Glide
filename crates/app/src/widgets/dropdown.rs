//! Bordered box + chevron dropdown, styled to match the reference's
//! "Input Source" control (a plain egui::ComboBox reads too different from
//! the mock — this draws the exact box/border/chevron treatment instead).

use crate::theme;
use crate::widgets::icons;
use egui::{Sense, Ui};

/// `current` is the currently-displayed label. `options` is the full list.
/// Returns `Some(index)` if the user picked a different option.
pub fn show(ui: &mut Ui, id_source: &str, current: &str, options: &[String]) -> Option<usize> {
    let desired_width = ui.available_width();
    let height = 34.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(desired_width, height),
        Sense::click(),
    );

    ui.painter()
        .rect_filled(rect, theme::rounding_small(), theme::BG_PANEL);
    ui.painter()
        .rect_stroke(rect, theme::rounding_small(), theme::border_stroke(), egui::StrokeKind::Inside);

    let text_rect = rect.shrink2(egui::vec2(12.0, 0.0));
    ui.painter().text(
        egui::pos2(text_rect.left(), text_rect.center().y),
        egui::Align2::LEFT_CENTER,
        current,
        egui::FontId::proportional(13.0),
        theme::TEXT_PRIMARY,
    );

    let chevron_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 16.0, rect.center().y),
        egui::vec2(14.0, 14.0),
    );
    icons::chevron(ui.painter(), chevron_rect, theme::TEXT_SECONDARY, false);

    let popup_id = ui.make_persistent_id(id_source);
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }

    let mut picked = None;
    egui::popup_below_widget(ui, popup_id, &response, egui::PopupCloseBehavior::CloseOnClick, |ui: &mut egui::Ui| {
        ui.set_min_width(rect.width());
        for (i, opt) in options.iter().enumerate() {
            if ui.selectable_label(opt == current, opt).clicked() {
                picked = Some(i);
                ui.memory_mut(|m| m.close_popup(popup_id));
            }
        }
    });

    picked
}
