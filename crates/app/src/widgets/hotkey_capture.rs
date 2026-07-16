//! A small "click to rebind" control: shows the current key combo as text
//! in a bordered box with an Edit button; clicking Edit puts it into a
//! listening state where the next keypress (with modifiers) is captured
//! and formatted back into the display string.
//!
//! This is intentionally simple — it captures combos while the egui
//! window has focus, via `ui.input()`. It is NOT the same mechanism as
//! the OTF feature's global `WH_KEYBOARD_LL` hook (which fires regardless
//! of focus) — this widget is only for entering a new binding while the
//! settings UI itself is focused. The resulting string still needs to be
//! handed off to whatever registers the actual global hotkey.

use crate::theme;
use egui::Ui;

pub struct HotkeyCaptureState {
    pub current: String,
    pub listening: bool,
}

impl HotkeyCaptureState {
    pub fn new(default: &str) -> Self {
        Self { current: default.to_string(), listening: false }
    }
}

/// Returns `Some(new_combo_string)` the moment a new combo is captured —
/// that's the point to fire the BACKEND HOOK that actually re-registers
/// the global hotkey.
pub fn show(ui: &mut Ui, label: &str, state: &mut HotkeyCaptureState) -> Option<String> {
    let mut captured = None;

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(12.5).color(theme::TEXT_PRIMARY));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button_label = if state.listening { "Press keys…" } else { "Edit" };
            if ui.small_button(button_label).clicked() {
                state.listening = !state.listening;
            }

            let box_size = egui::vec2(140.0, 26.0);
            let (rect, _) = ui.allocate_exact_size(box_size, egui::Sense::hover());
            ui.painter().rect_filled(rect, theme::rounding_small(), theme::BG_PANEL);
            ui.painter().rect_stroke(rect, theme::rounding_small(), theme::border_stroke(), egui::StrokeKind::Inside);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &state.current,
                egui::FontId::monospace(12.0),
                theme::TEXT_PRIMARY,
            );
        });
    });

    if state.listening {
        let combo = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    Some(format_combo(*modifiers, *key))
                }
                _ => None,
            })
        });
        if let Some(combo) = combo {
            state.current = combo.clone();
            state.listening = false;
            captured = Some(combo);
        }
    }

    captured
}

fn format_combo(modifiers: egui::Modifiers, key: egui::Key) -> String {
    let mut parts = vec![];
    if modifiers.ctrl { parts.push("Ctrl"); }
    if modifiers.shift { parts.push("Shift"); }
    if modifiers.alt { parts.push("Alt"); }
    parts.push(key.name());
    parts.join("+")
}
