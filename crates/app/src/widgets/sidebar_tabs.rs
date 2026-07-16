//! The left-hand vertical tab list in the settings panel. egui has no
//! built-in vertical tab widget — this is a manually-built selectable
//! column, matching the reference's icon+label rows with a light-purple
//! fill on the selected row.

use crate::theme;
use crate::widgets::icons;
use egui::{Sense, Ui};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsTab {
    General,
    Audio,
    Shortcuts,
    Advance,
}

impl SettingsTab {
    pub const ALL: [SettingsTab; 4] = [
        SettingsTab::General,
        SettingsTab::Audio,
        SettingsTab::Shortcuts,
        SettingsTab::Advance,
    ];

    fn label(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Audio => "Audio",
            SettingsTab::Shortcuts => "Shortcuts",
            SettingsTab::Advance => "Advance",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            SettingsTab::General => icons::GEAR,
            SettingsTab::Audio => icons::SPEAKER_HIGH,
            SettingsTab::Shortcuts => icons::KEYBOARD,
            SettingsTab::Advance => icons::LIGHTNING,
        }
    }
}

/// Returns `Some(tab)` if the user clicked a different tab than `current`.
pub fn show(ui: &mut Ui, current: SettingsTab) -> Option<SettingsTab> {
    let mut clicked = None;
    ui.vertical(|ui| {
        ui.set_width(140.0);
        for tab in SettingsTab::ALL {
            let selected = tab == current;
            let row_height = 34.0;
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), row_height),
                Sense::click(),
            );

            if selected {
                ui.painter().rect_filled(
                    rect.shrink(2.0),
                    theme::rounding_row(),
                    theme::BG_SIDEBAR_SELECTED,
                );
            } else if response.hovered() {
                ui.painter().rect_filled(
                    rect.shrink(2.0),
                    theme::rounding_row(),
                    theme::BG_CONTROL,
                );
            }

            let color = if selected {
                theme::ACCENT_TEXT
            } else {
                theme::TEXT_SECONDARY
            };
            ui.painter().text(
                egui::pos2(rect.left() + 24.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                tab.glyph(),
                egui::FontId::proportional(16.0),
                color,
            );

            ui.painter().text(
                egui::pos2(rect.left() + 42.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                tab.label(),
                egui::FontId::proportional(13.0),
                if selected { theme::ACCENT_TEXT } else { theme::TEXT_PRIMARY },
            );

            if response.clicked() {
                clicked = Some(tab);
            }
        }
    });
    clicked
}
