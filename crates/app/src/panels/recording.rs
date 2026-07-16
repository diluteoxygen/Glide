use crate::config::{AppConfig, Format};
use crate::theme;
use crate::ui::{group, row, section_label};
use crate::widgets::{segmented, toggle};
use egui::{ComboBox, Ui};

pub fn show(ui: &mut Ui, cfg: &mut AppConfig) {
    section_label(ui, "Container");
    group(ui, |ui| {
        let cur = if cfg.format == Format::Mkv { 0 } else { 1 };
        row(ui, "Recording format", Some(if cfg.format == Format::Mkv {
            "MKV is crash-safe — a power loss keeps the file playable."
        } else {
            "MP4 is universally compatible, used for the final export."
        }), |ui| {
            if let Some(i) = segmented::show(ui, "fmt", &["MKV", "MP4"], cur) {
                cfg.format = if i == 0 { Format::Mkv } else { Format::Mp4 };
            }
        });
        ui.separator();
        row(ui, "Export an MP4 copy after recording", Some("Records MKV first, then remuxes a tidy MP4 when you stop."), |ui| {
            toggle::show(ui, "export_mp4", &mut cfg.export_mp4_copy);
        });
    });

    ui.add_space(14.0);
    section_label(ui, "Quality");
    group(ui, |ui| {
        row(ui, "Resolution", None, |ui| {
            ComboBox::from_id_source("res")
                .selected_text(&cfg.resolution)
                .show_ui(ui, |ui| {
                    for r in ["720p", "1080p", "1440p", "4K"] {
                        ui.selectable_value(&mut cfg.resolution, r.to_string(), r);
                    }
                });
        });
        ui.separator();
        row(ui, "Frame rate", None, |ui| {
            let cur = if cfg.framerate >= 60 { 1 } else { 0 };
            if let Some(i) = segmented::show(ui, "fps", &["30 fps", "60 fps"], cur) {
                cfg.framerate = if i == 0 { 30 } else { 60 };
            }
        });
        ui.separator();
        row(ui, "Quality", None, |ui| {
            ComboBox::from_id_source("qual")
                .selected_text(format!("{} quality", cfg.quality))
                .show_ui(ui, |ui| {
                    for q in ["Smooth", "Balanced", "Crisp", "Max"] {
                        ui.selectable_value(&mut cfg.quality, q.to_string(), q);
                    }
                });
        });
    });

    ui.add_space(14.0);
    section_label(ui, "Capture");
    group(ui, |ui| {
        row(ui, "Capture the cursor", Some("Include the pointer in the recording."), |ui| {
            toggle::show(ui, "cursor", &mut cfg.capture_cursor);
        });
        ui.separator();
        row(ui, "Highlight cursor clicks", Some("Render a soft ripple on each click."), |ui| {
            toggle::show(ui, "clicks", &mut cfg.capture_clicks);
        });
        ui.separator();
        row(ui, "Hardware acceleration", Some("Use VideoToolbox / NVENC when available for lower CPU."), |ui| {
            toggle::show(ui, "hwaccel", &mut cfg.hardware_accel);
        });
    });

    let _ = theme::TEXT1; // keep import used
}
