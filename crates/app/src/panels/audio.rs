use crate::config::AppConfig;
use crate::ui::{group, row, section_label};
use crate::widgets::{level, toggle};
use crate::{icons, theme};
use egui::{ComboBox, Ui};

pub fn show(ui: &mut Ui, cfg: &mut AppConfig) {
    section_label(ui, "Microphone");
    group(ui, |ui| {
        row(ui, "Input source", None, |ui| {
            ComboBox::from_id_source("micsrc")
                .selected_text(&cfg.mic_source)
                .show_ui(ui, |ui| {
                    for d in ["MacBook Pro Microphone", "AirPods Pro", "Blue Yeti USB", "System Default"] {
                        ui.selectable_value(&mut cfg.mic_source, d.to_string(), d);
                    }
                });
        });
        ui.separator();
        row(ui, "Input level", None, |ui| {
            level::show(ui, cfg.mic_enabled);
        });
        ui.separator();
        row(ui, "Record microphone", Some("Off = system audio only."), |ui| {
            toggle::show(ui, "mic", &mut cfg.mic_enabled);
        });
    });

    ui.add_space(14.0);
    section_label(ui, "System");
    group(ui, |ui| {
        row(ui, "Record system audio", Some("Capture everything playing on this Mac via loopback."), |ui| {
            toggle::show(ui, "sys", &mut cfg.system_audio);
        });
    });

    let _ = (icons::image, theme::TEXT1);
}
