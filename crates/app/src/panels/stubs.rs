//! General / Shortcuts / Advance tabs. The reference wireframe only fully
//! specifies the Audio tab's content — these three are placeholders on
//! purpose. Don't invent a full design for them here; flag with the
//! product owner what each should actually contain before building it out.

use crate::theme;
use egui::Ui;

pub fn general(ui: &mut Ui) {
    placeholder(ui, "General settings — not yet specified.");
}

pub fn shortcuts(ui: &mut Ui) {
    placeholder(ui, "Shortcuts settings — not yet specified.");
}

pub fn advance(ui: &mut Ui) {
    placeholder(ui, "Advanced settings — not yet specified.");
}

fn placeholder(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).size(13.0).color(theme::TEXT_SECONDARY));
}
