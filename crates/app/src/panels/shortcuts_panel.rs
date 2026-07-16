use crate::theme;
use crate::widgets::hotkey_capture::{self, HotkeyCaptureState};
use egui::Ui;

pub struct ShortcutsPanelState {
    pub start_stop: HotkeyCaptureState,
    pub pause_resume: HotkeyCaptureState,
}

impl Default for ShortcutsPanelState {
    fn default() -> Self {
        Self {
            start_stop: HotkeyCaptureState::new("Ctrl+Shift+R"),
            pause_resume: HotkeyCaptureState::new("Ctrl+Shift+P"),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut ShortcutsPanelState) {
    ui.vertical(|ui| {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Global Hotkeys")
                .size(13.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(6.0);

        egui::Frame::NONE
            .fill(theme::BG_PANEL)
            .stroke(theme::border_stroke())
            .corner_radius(theme::rounding_small())
            .inner_margin(egui::vec2(14.0, 10.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    if let Some(new_bind) = hotkey_capture::show(ui, "Start/Stop Recording", &mut state.start_stop) {
                        // BACKEND HOOK: Re-register start/stop hotkey
                        let _ = new_bind;
                    }
                    ui.add_space(10.0);
                    if let Some(new_bind) = hotkey_capture::show(ui, "Pause/Resume Recording", &mut state.pause_resume) {
                        // BACKEND HOOK: Re-register pause/resume hotkey
                        let _ = new_bind;
                    }
                });
            });

        ui.add_space(18.0);
        ui.label(
            egui::RichText::new("OTF Gestures (Live Mode)")
                .size(13.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(6.0);

        egui::Frame::NONE
            .fill(theme::BG_PANEL)
            .stroke(theme::border_stroke())
            .corner_radius(theme::rounding_small())
            .inner_margin(egui::vec2(14.0, 10.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    readonly_hotkey_row(ui, "Follow cursor toggle", "Double-tap Shift or Ctrl");
                    ui.add_space(10.0);
                    readonly_hotkey_row(ui, "Zoom in / out", "Shift + Mouse Scroll");
                });
            });
    });
}

fn readonly_hotkey_row(ui: &mut Ui, label: &str, bind: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(12.5).color(theme::TEXT_PRIMARY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(bind).size(12.0).color(theme::TEXT_SECONDARY));
        });
    });
}
