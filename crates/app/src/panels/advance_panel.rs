use crate::theme;
use crate::widgets::{dropdown, toggle_switch};
use egui::Ui;

pub struct AdvancePanelState {
    pub encoder: Option<String>,
    pub ring_buffer_capacity: u32,
    pub verbose_logging: bool,
    pub zoom_intensity: f32,
    pub available_encoders: Vec<String>,
}

impl Default for AdvancePanelState {
    fn default() -> Self {
        Self {
            encoder: None,
            ring_buffer_capacity: 4,
            verbose_logging: false,
            zoom_intensity: 0.5,
            available_encoders: vec![
                "libx264 (Software)".to_string(),
                "h264_nvenc (Hardware)".to_string(),
                "h264_qsv (Intel QuickSync)".to_string(),
            ],
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AdvancePanelState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Encoder Override")
                    .size(13.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            ui.add_space(6.0);

            let encoders = state.available_encoders.clone();
            let current = state.encoder.clone().unwrap_or_else(|| "libx264 (Software)".to_string());
            if let Some(idx) = dropdown::show(ui, "encoder_dropdown", &current, &encoders) {
                state.encoder = Some(encoders[idx].clone());
            }
            
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Detected hardware encoder: NVIDIA NVENC HEVC").size(11.0).color(theme::TEXT_SECONDARY));

            ui.add_space(18.0);
            ui.label(
                egui::RichText::new("OTF Zoom Solver")
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
                    ui.horizontal(|ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Zoom Intensity").size(13.0).color(theme::TEXT_PRIMARY));
                            ui.label(egui::RichText::new("Higher intensity makes zoom feel punchier").size(11.0).color(theme::TEXT_SECONDARY));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(egui::Slider::new(&mut state.zoom_intensity, 0.1..=1.0).show_value(false));
                        });
                    });
                });

            ui.add_space(18.0);
            ui.label(
                egui::RichText::new("System Tuning")
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
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Ring Buffer Capacity").size(13.0).color(theme::TEXT_PRIMARY));
                                ui.label(egui::RichText::new("Frames buffered before encoder backpressure drops them").size(11.0).color(theme::TEXT_SECONDARY));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add(egui::Slider::new(&mut state.ring_buffer_capacity, 1..=10));
                            });
                        });
                        
                        ui.add_space(10.0);
                        
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Verbose Logging").size(13.0).color(theme::TEXT_PRIMARY));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                toggle_switch::show(ui, "verbose_log_toggle", &mut state.verbose_logging);
                            });
                        });
                        
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);
                        
                        if ui.button("Reset to Defaults").clicked() {
                            *state = AdvancePanelState::default();
                        }
                    });
                });
        });
    });
}
