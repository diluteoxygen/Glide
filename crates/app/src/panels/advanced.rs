use crate::config::{AppConfig, Encoder};
use crate::theme;
use crate::ui::{group, row, section_label};
use crate::widgets::toggle;
use egui::{ComboBox, RichText, Slider, Ui};

pub fn show(ui: &mut Ui, cfg: &mut AppConfig) {
    section_label(ui, "Encoder");
    group(ui, |ui| {
        row(ui, "Video encoder", None, |ui| {
            ComboBox::from_id_source("enc")
                .selected_text(enc_label(cfg.encoder))
                .show_ui(ui, |ui| {
                    for e in [
                        Encoder::Auto,
                        Encoder::VideoToolbox,
                        Encoder::Nvenc,
                        Encoder::Qsv,
                        Encoder::Amf,
                        Encoder::Vaapi,
                        Encoder::X264,
                    ] {
                        ui.selectable_value(&mut cfg.encoder, e, enc_label(e));
                    }
                });
        });
        ui.separator();
        row(ui, "Detected", None, |ui| {
            ui.label(RichText::new("VideoToolbox (H.265) · probed at launch").size(12.0).color(theme::TEXT2));
        });
    });

    ui.add_space(14.0);
    section_label(ui, "Pipeline");
    group(ui, |ui| {
        row(ui, "Zoom intensity", Some("Mapped to the camera solver's spring damping."), |ui| {
            ui.add(Slider::new(&mut cfg.zoom_intensity, 0..=100).clamp_to_range(true).text("%"));
        });
        ui.separator();
        row(ui, "Ring buffer", Some("Frames buffered before the encoder; higher = smoother under load."), |ui| {
            ui.add(Slider::new(&mut cfg.ring_buffer, 1..=16).clamp_to_range(true).text("fr"));
        });
    });

    ui.add_space(14.0);
    section_label(ui, "Diagnostics");
    group(ui, |ui| {
        row(ui, "Verbose logging", Some("Write detailed pipeline traces to the log file."), |ui| {
            toggle::show(ui, "verbose", &mut cfg.verbose_logging);
        });
    });

    ui.add_space(12.0);
    if ui.button("Reset Advanced to defaults").clicked() {
        cfg.encoder = Encoder::Auto;
        cfg.ring_buffer = 4;
        cfg.zoom_intensity = 65;
        cfg.verbose_logging = false;
        cfg.save();
    }
}

fn enc_label(e: Encoder) -> &'static str {
    match e {
        Encoder::Auto => "Auto (recommended)",
        Encoder::VideoToolbox => "VideoToolbox · Apple GPU",
        Encoder::Nvenc => "NVENC · NVIDIA",
        Encoder::Qsv => "Quick Sync · Intel",
        Encoder::Amf => "AMF · AMD",
        Encoder::Vaapi => "VAAPI · Linux",
        Encoder::X264 => "Software · x264",
    }
}
