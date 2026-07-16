use crate::config::AppConfig;
use crate::theme;
use crate::ui::{group, row, section_label};
use crate::widgets::{segmented, toggle};
use crate::{icons, ui as ghui};
use egui::{RichText, Ui};

pub fn show(ui: &mut Ui, cfg: &mut AppConfig) {
    section_label(ui, "Output");
    group(ui, |ui| {
        ui.horizontal(|ui| {
            icons::draw(ui, "folder", 18.0, theme::TEXT2);
            ui.vertical(|ui| {
                ui.label(RichText::new("Save recordings to").size(13.0).color(theme::TEXT1));
                ui.label(RichText::new(&cfg.output_path).size(11.5).color(theme::TEXT2));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Change…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().set_directory(&cfg.output_path).pick_folder() {
                        cfg.output_path = p.to_string_lossy().into_owned();
                        cfg.save();
                    }
                }
                if ui.button("Reveal").clicked() {
                    let _ = std::fs::create_dir_all(&cfg.output_path);
                    let _ = open::that(&cfg.output_path);
                }
            });
        });
    });

    ui.add_space(14.0);
    section_label(ui, "Behavior");
    group(ui, |ui| {
        row(ui, "Show recording HUD", Some("A floating overlay with elapsed time & zoom level — never burned into the video."), |ui| {
            toggle::show(ui, "hud", &mut cfg.show_hud);
        });
        ui.separator();
        row(ui, "Launch Glide at login", None, |ui| {
            if toggle::show(ui, "login", &mut cfg.launch_at_login) {
                set_login(cfg.launch_at_login);
                cfg.save();
            }
        });
        ui.separator();
        row(
            ui,
            "Minimize to tray when closed",
            Some(if cfg.minimize_to_tray {
                "ON — closing hides Glide to the menu bar instead of quitting."
            } else {
                "OFF — closing fully quits Glide. It will not hide to the tray."
            }),
            |ui| {
                if toggle::show(ui, "tray", &mut cfg.minimize_to_tray) {
                    cfg.save();
                }
            },
        );
        ui.separator();
        let cur = match cfg.countdown { 0 => 0, 3 => 1, _ => 2 };
        row(ui, "Countdown before recording", None, |ui| {
            if let Some(i) = segmented::show(ui, "count", &["Off", "3s", "5s"], cur) {
                cfg.countdown = [0, 3, 5][i];
            }
        });
    });

    ui.add_space(14.0);
    section_label(ui, "After recording");
    group(ui, |ui| {
        row(ui, "Show post-recording summary", Some("File size, duration and a “Show in folder” button when you stop."), |ui| {
            toggle::show(ui, "summary", &mut cfg.post_summary);
        });
    });

}

fn set_login(enable: bool) {
    let exe = std::env::current_exe().ok();
    let Some(exe) = exe else { return };
    let al = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Glide")
        .set_app_path(&exe.to_string_lossy())
        .build().unwrap();
    let res = if enable { al.enable() } else { al.disable() };
    if let Err(e) = res {
        eprintln!("[glide] launch-at-login: {e}");
    }
}
