//! Click "Edit" to listen, then press a combo to capture it.

use crate::theme;
use egui::{Event, Key, Modifiers, RichText, Ui};

pub enum HotkeyAction {
    None,
    ToggleListen,
    Captured(String),
}

pub fn show(ui: &mut Ui, label: &str, value: &str, listening: bool) -> HotkeyAction {
    let mut action = HotkeyAction::None;

    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0).color(theme::TEXT1));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let btn = if listening {
                ui.add(egui::Button::new("Press keys…").fill(theme::ACCENT).stroke(egui::Stroke::NONE))
            } else {
                ui.button("Edit")
            };
            if btn.clicked() {
                action = HotkeyAction::ToggleListen;
            }

            let (r, _) = ui.allocate_exact_size(egui::vec2(110.0, 26.0), egui::Sense::hover());
            let p = ui.painter();
            p.rect_filled(r, egui::Rounding::same(6.0), theme::CONTROL);
            p.rect_stroke(r, egui::Rounding::same(6.0), theme::hair_stroke());
            let text = if listening { "Listening…" } else { value };
            p.text(r.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::monospace(12.0), theme::TEXT1);
        });
    });

    if listening {
        if let Some(combo) = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                Event::Key { key, pressed: true, modifiers, .. } => Some(format_combo(*modifiers, *key)),
                _ => None,
            })
        }) {
            action = HotkeyAction::Captured(combo);
        }
    }
    action
}

fn format_combo(m: Modifiers, key: Key) -> String {
    let mut parts: Vec<String> = Vec::new();
    if m.command { parts.push("⌘".to_string()); }
    if m.ctrl { parts.push("⌃".to_string()); }
    if m.alt { parts.push("⌥".to_string()); }
    if m.shift { parts.push("⇧".to_string()); }
    parts.push(format!("{:?}", key));
    parts.join("")
}
