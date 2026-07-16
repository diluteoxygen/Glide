use crate::theme;
use crate::widgets::{icons, toggle_switch};
use egui::Ui;

pub struct GeneralPanelState {
    pub output_path: String,
    pub show_recording_hud: bool,
    pub launch_on_system_start: bool,
    pub minimize_to_tray_on_close: bool,
    pub export_mp4_after_recording: bool,
    pub countdown_seconds: u8, // 0 = off
}

impl Default for GeneralPanelState {
    fn default() -> Self {
        Self {
            output_path: "Documents/Glide recordings".to_string(),
            show_recording_hud: true,
            launch_on_system_start: false,
            minimize_to_tray_on_close: true,
            export_mp4_after_recording: false,
            countdown_seconds: 3,
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut GeneralPanelState) {
    ui.vertical(|ui| {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Output")
                .size(13.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            icons::draw(ui, icons::FOLDER, 16.0, theme::TEXT_SECONDARY);
            ui.label(
                egui::RichText::new(&state.output_path)
                    .size(13.0)
                    .color(theme::TEXT_PRIMARY),
            );
            if ui.small_button("Change").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    state.output_path = path.display().to_string();
                }
            }
        });
        
        ui.add_space(8.0);
        let disk_info = "Estimated 14 hrs remaining (120 GB free)"; // Placeholder until sysinfo is fully integrated
        ui.label(egui::RichText::new(disk_info).size(11.0).color(theme::TEXT_SECONDARY));
        // BACKEND HOOK: If the drive is a slow network drive, show a warning here!

        ui.add_space(18.0);
        ui.label(
            egui::RichText::new("Behavior")
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
                    toggle_row(ui, "Show recording HUD", "Display a live widget on screen while recording", "hud_toggle", &mut state.show_recording_hud);
                    ui.add_space(8.0);
                    toggle_row(ui, "Launch on system start", "Automatically start Glide when you log in", "launch_toggle", &mut state.launch_on_system_start);
                    ui.add_space(8.0);
                    toggle_row(ui, "Minimize to tray on close", "Keep Glide running in the background", "tray_toggle", &mut state.minimize_to_tray_on_close);
                    ui.add_space(8.0);
                    toggle_row(ui, "Export MP4 after recording", "Automatically remux MKV to MP4 when finished", "mp4_toggle", &mut state.export_mp4_after_recording);
                    
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Countdown before recording").size(13.0).color(theme::TEXT_PRIMARY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(egui::Slider::new(&mut state.countdown_seconds, 0..=10).text("sec"));
                        });
                    });
                });
            });
            
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            icons::draw(ui, icons::WARNING, 14.0, theme::TEXT_SECONDARY);
            ui.label(egui::RichText::new("Accessibility keys: OK").size(11.0).color(theme::TEXT_SECONDARY));
        });
    });
}

fn toggle_row(ui: &mut Ui, title: &str, desc: &str, id: &str, val: &mut bool) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(title).size(13.0).strong().color(theme::TEXT_PRIMARY));
            ui.label(egui::RichText::new(desc).size(11.0).color(theme::TEXT_SECONDARY));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            toggle_switch::show(ui, id, val);
        });
    });
}
