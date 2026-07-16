use crate::config::AppConfig;
use crate::ui::{group, section_label};
use crate::widgets::hotkey::{self, HotkeyAction};
use egui::{RichText, Ui};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HotkeyField {
    StartStop,
    PauseResume,
}

pub fn show(ui: &mut Ui, cfg: &mut AppConfig, listening: &mut Option<HotkeyField>) {
    section_label(ui, "Global hotkeys");
    group(ui, |ui| {
        let lis_start = *listening == Some(HotkeyField::StartStop);
        match hotkey::show(ui, "Start / Stop recording", &cfg.hotkey_start_stop, lis_start) {
            HotkeyAction::ToggleListen => {
                *listening = if lis_start { None } else { Some(HotkeyField::StartStop) }
            }
            HotkeyAction::Captured(c) => {
                cfg.hotkey_start_stop = c;
                *listening = None;
                cfg.save();
            }
            _ => {}
        }
        ui.separator();
        let lis_pause = *listening == Some(HotkeyField::PauseResume);
        match hotkey::show(ui, "Pause / Resume", &cfg.hotkey_pause_resume, lis_pause) {
            HotkeyAction::ToggleListen => {
                *listening = if lis_pause { None } else { Some(HotkeyField::PauseResume) }
            }
            HotkeyAction::Captured(c) => {
                cfg.hotkey_pause_resume = c;
                *listening = None;
                cfg.save();
            }
            _ => {}
        }
    });

    ui.add_space(14.0);
    section_label(ui, "Live-zoom gestures · fixed");
    ui.label(
        RichText::new("Part of the gesture engine — can't be remapped, always active in Live mode.")
            .size(11.5)
            .color(crate::theme::TEXT2),
    );
    ui.add_space(4.0);
    group(ui, |ui| {
        gesture(ui, "⇧⇧", "Double-tap Shift", "Engage follow / reset to default zoom");
        ui.separator();
        gesture(ui, "⌃⌃", "Double-tap Control", "Return to 1.0×");
        ui.separator();
        gesture(ui, "⇧ + Scroll", "Shift + Scroll", "Adjust zoom level directly");
    });
}

fn gesture(ui: &mut Ui, combo: &str, name: &str, effect: &str) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(
            RichText::new(combo).font(egui::FontId::monospace(11.5)).color(crate::theme::TEXT1),
        ));
        ui.label(RichText::new(name).size(13.0).color(crate::theme::TEXT1));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(effect).size(11.5).color(crate::theme::TEXT2));
        });
    });
}
