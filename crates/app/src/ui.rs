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
        .inner_margin(Margin { left: 14.0, right: 14.0, top: 4.0, bottom: 4.0 })
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
