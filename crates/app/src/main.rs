#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pipeline;
use pipeline::PipelineHandle;

use eframe::egui;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use tray_icon::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, TrayIconEvent,
};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::thread;

pub struct GlideApp {
    output_dir: String,
    is_otf: bool,
    no_overlay: bool,
    is_recording: bool,
    
    // Channels & Handles
    hotkey_manager: GlobalHotKeyManager,
    hotkey: HotKey,
    tray_menu: Menu,
    toggle_i: MenuItem,
    quit_i: MenuItem,
    
    pipeline: Option<PipelineHandle>,
}

impl GlideApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Setup Tray Icon
        let tray_menu = Menu::new();
        let toggle_i = MenuItem::new("Show/Hide", true, None);
        let quit_i = MenuItem::new("Quit", true, None);
        tray_menu.append_items(&[&toggle_i, &PredefinedMenuItem::separator(), &quit_i]).unwrap();
        
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu.clone()))
            .with_tooltip("Glide Screen Recorder")
            // .with_icon(...) // TODO: Add icon later
            .build()
            .unwrap();
            
        // Keep tray_icon alive by leaking it or storing it (TrayIcon struct doesn't have a Drop that destroys it immediately, but storing it is better)
        Box::leak(Box::new(tray_icon));

        // Setup Global Hotkey (Ctrl+Shift+R)
        let hotkey_manager = GlobalHotKeyManager::new().unwrap();
        let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyR);
        hotkey_manager.register(hotkey).unwrap();

        Self {
            output_dir: ".".to_string(),
            is_otf: true,
            no_overlay: false,
            is_recording: false,
            hotkey_manager,
            hotkey,
            tray_menu,
            toggle_i,
            quit_i,
            pipeline: None,
        }
    }
    
    fn toggle_recording(&mut self) {
        if self.is_recording {
            // Stop recording
            if let Some(pipe) = self.pipeline.take() {
                pipe.stop_signal.store(true, Ordering::Relaxed);
            }
            self.is_recording = false;
        } else {
            // Start recording
            match pipeline::start_recording(&self.output_dir, self.is_otf, self.no_overlay) {
                Ok(handle) => {
                    self.pipeline = Some(handle);
                    self.is_recording = true;
                }
                Err(e) => {
                    tracing::error!("Failed to start recording: {}", e);
                    // Could show an egui error dialog here
                }
            }
        }
    }
}

impl eframe::App for GlideApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process Tray Events
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if event.click_type == tray_icon::ClickType::Left {
                // Not supported well in pure tray_icon sometimes without a window reference, 
                // but eframe handles visibility if we just tell it
                // Actually eframe doesn't have a direct "hide window" from inside `update` unless we use ViewportCommand.
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
        
        // Process Menu Events
        if let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            if event.id == self.quit_i.id() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if event.id == self.toggle_i.id() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        // Process Hotkey Events
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.id == self.hotkey.id() {
                if event.state == global_hotkey::HotKeyState::Released {
                    self.toggle_recording();
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Glide Screen Recorder");
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Output Directory:");
                if ui.button("Select...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.output_dir = path.to_string_lossy().to_string();
                    }
                }
            });
            ui.label(format!("Path: {}", self.output_dir));
            
            ui.separator();
            ui.checkbox(&mut self.is_otf, "Enable Live Zoom (OTF)");
            if self.is_otf {
                ui.checkbox(&mut self.no_overlay, "Disable Screen Dimming Overlay");
            }
            
            ui.separator();
            ui.horizontal(|ui| {
                if !self.is_recording {
                    if ui.button("▶ Start Recording").clicked() {
                        self.toggle_recording();
                    }
                } else {
                    if ui.button("⏹ Stop Recording").clicked() {
                        self.toggle_recording();
                    }
                }
            });
            
            ui.label("Global Hotkey: Ctrl+Shift+R to start/stop");
        });
        
        // Request a repaint so the channel events get polled quickly.
        // In a real app we might want to block or sleep, but eframe continuous mode is easy.
        ctx.request_repaint();
    }
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_title("Glide"),
        ..Default::default()
    };

    eframe::run_native(
        "Glide Screen Recorder",
        options,
        Box::new(|cc| Box::new(GlideApp::new(cc))),
    )
}
