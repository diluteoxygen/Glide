//! Main window: toolbar, status footer, animated settings panel, and the
//! fixed close→tray logic.

use crate::config::{AppConfig, Format, Mode};
use crate::icons;
use crate::panels::{self, Tab};
use crate::panels::shortcuts::HotkeyField;
use crate::theme;
use crate::ui::{group, row, section_label};
use crate::widgets::{segmented, toggle, IconButton};
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Layout, Margin, Pos2, Rect, Rounding, Sense,
    Stroke, Vec2, ViewportCommand,
};
use std::time::{Duration, Instant};
use tray_icon::TrayIcon;

const COLLAPSED: f32 = 150.0;
const EXPANDED: f32 = 420.0;
const DEFAULT_W: f32 = 660.0;

pub struct GlideApp {
    cfg: AppConfig,
    tray: TrayIcon,
    tab: Tab,
    settings_open: bool,
    listening: Option<HotkeyField>,

    is_recording: bool,
    started_at: Option<Instant>,

    last_sent: Option<Vec2>,

    // cached free-space readout (time, free_gb, est_minutes)
    disk: Option<(Instant, f64, u64)>,
    toast: Option<(Instant, String, Color32)>,

    pipeline: Option<crate::pipeline::PipelineHandle>,
}

impl GlideApp {
    pub fn new(cfg: AppConfig, tray: TrayIcon) -> Self {
        Self {
            cfg,
            tray,
            tab: Tab::Recording,
            settings_open: false,
            listening: None,
            is_recording: false,
            started_at: None,
            last_sent: None,
            disk: None,
            toast: None,
            pipeline: None,
        }
    }

    // ── THE headline fix ────────────────────────────────────────────────
    // When the OS close button is pressed, egui sets `close_requested`. The
    // root viewport would then exit — UNLESS we send `CancelClose`. We gate
    // that on the *actual* setting value, which is what round-2 got wrong
    // (it always hid to tray regardless of the toggle).
    fn handle_close(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if !close_requested {
            return;
        }

        if crate::tray::FORCE_QUIT.swap(false, std::sync::atomic::Ordering::SeqCst) {
            // Tray "Quit" → let the app exit. Do nothing.
            return;
        }

        if self.cfg.minimize_to_tray {
            // Setting ON → hide to tray instead of quitting.
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
            self.toast("Hid to the menu bar", theme::GREEN);
        }
        // else: setting OFF → do nothing → Glide fully quits. (The fix.)
    }

    fn toast(&mut self, msg: &str, color: Color32) {
        self.toast = Some((Instant::now(), msg.to_string(), color));
    }

    fn refresh_disk(&mut self) {
        let needs = match self.disk {
            None => true,
            Some((t, _, _)) => t.elapsed() > Duration::from_secs(5),
        };
        if !needs {
            return;
        }
        use sysinfo::Disks;
        let disks = Disks::new_with_refreshed_list();
        // pick the disk whose mount point is a prefix of the output path (else first)
        let target = std::path::Path::new(&self.cfg.output_path);
        let pick = disks
            .list()
            .iter()
            .find(|d| target.starts_with(d.mount_point()))
            .or_else(|| disks.list().first());
        if let Some(d) = pick {
            let free_b = d.available_space();
            let free_gb = free_b as f64 / 1_073_741_824.0;
            // ~6 Mbps → 0.75 MB/s → minutes
            let mins = (free_b as f64 / (0.75 * 1_048_576.0)) / 60.0;
            self.disk = Some((Instant::now(), free_gb, mins as u64));
        }
    }

    fn apply_sizing(&mut self, ctx: &egui::Context, target_h: f32) {
        // Don't fight the OS while maximized (the round-1 maximize bug).
        let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
        if maximized {
            self.last_sent = None;
            return;
        }
        let w = ctx
            .input(|i| i.viewport().inner_rect.map(|r| r.width()))
            .unwrap_or(DEFAULT_W);
        let desired = Vec2::new(w, target_h);
        let changed = match self.last_sent {
            Some(last) => (last - desired).length() > 0.5,
            None => true,
        };
        if changed {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(desired));
            self.last_sent = Some(desired);
        }
    }

    fn toggle_record(&mut self) {
        if self.is_recording {
            self.is_recording = false;
            let secs = self.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);
            self.started_at = None;
            let ext = if self.cfg.export_mp4_copy { "mp4" } else { self.cfg.format.as_ext() };
            if self.cfg.post_summary {
                self.toast(
                    &format!("Saved · {}s · .{}", secs, ext),
                    theme::GREEN,
                );
            }
            // BACKEND HOOK: stop the pipeline; if export_mp4_copy, run the remux.
            if let Some(pipe) = self.pipeline.take() {
                pipe.stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        } else {
            self.is_recording = true;
            self.started_at = Some(Instant::now());
            if self.settings_open {
                self.settings_open = false;
            }
            if self.cfg.countdown > 0 {
                self.toast(&format!("Recording in {}s…", self.cfg.countdown), theme::ACCENT);
            }
            // BACKEND HOOK: start the pipeline in Raw or Live mode.
            let is_otf = self.cfg.mode == Mode::Live;
            let no_overlay = false; // Assuming default false since it's not in the new UI config
            match crate::pipeline::start_recording(&self.cfg.output_path, is_otf, no_overlay) {
                Ok(handle) => {
                    self.pipeline = Some(handle);
                }
                Err(e) => {
                    tracing::error!("Failed to start recording: {}", e);
                    self.toast("Failed to start recording", theme::RED);
                    self.is_recording = false;
                    self.started_at = None;
                }
            }
        }
    }
}

impl Format {
    fn as_ext(self) -> &'static str {
        match self {
            Format::Mkv => "mkv",
            Format::Mp4 => "mp4",
        }
    }
}

impl eframe::App for GlideApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_close(ctx); // <-- minimize-to-tray fix lives here
        crate::tray::pump(ctx);

        // pending record toggle from the tray menu
        if crate::tray::PENDING_RECORD.swap(false, std::sync::atomic::Ordering::SeqCst) {
            self.toggle_record();
        }

        if self.is_recording {
            ctx.request_repaint_after(Duration::from_millis(250));
        }

        // expand/collapse spring
        let progress = ctx.animate_bool(egui::Id::new("expand"), self.settings_open);
        if progress > 0.001 && progress < 0.999 {
            ctx.request_repaint();
        }
        let target_h = COLLAPSED + EXPANDED * progress;
        self.apply_sizing(ctx, target_h);
        self.refresh_disk();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::WINDOW).inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                self.toolbar(ui);
                ui.add_space(10.0);
                self.status_row(ui);

                if progress > 0.01 {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    let avail = EXPANDED * progress;
                    ui.scope(|ui| {
                        ui.set_max_height(avail.max(0.0));
                        ui.set_clip_rect(ui.max_rect());
                        self.settings(ui, progress);
                    });
                }
            });

        self.draw_toast(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cfg.save();
    }
}

// ── layout pieces ──────────────────────────────────────────────────────
impl GlideApp {
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Mode
            let cur = if self.cfg.mode == Mode::Live { 1 } else { 0 };
            if let Some(i) = segmented::show(ui, "mode", &["Raw", "Live"], cur) {
                self.cfg.mode = if i == 0 { Mode::Raw } else { Mode::Live };
            }

            ui.add_space(6.0);
            self.format_chip(ui);

            ui.separator();

            // Mic
            if ui
                .add(
                    IconButton {
                        icon: if self.cfg.mic_enabled { "mic" } else { "micOff" },
                        active: !self.cfg.mic_enabled,
                        danger: true,
                        tooltip: Some(if self.cfg.mic_enabled {
                            "Mute microphone — record system audio only"
                        } else {
                            "Unmute microphone"
                        }),
                        ..Default::default()
                    },
                )
                .clicked()
            {
                self.cfg.mic_enabled = !self.cfg.mic_enabled;
                self.toast(
                    if self.cfg.mic_enabled { "Microphone on" } else { "Muted — system audio only" },
                    theme::TEXT1,
                );
            }

            // Reveal folder
            if ui
                .add(IconButton {
                    icon: "reveal",
                    tooltip: Some("Reveal output folder"),
                    ..Default::default()
                })
                .clicked()
            {
                let _ = std::fs::create_dir_all(&self.cfg.output_path);
                let _ = open::that(&self.cfg.output_path);
                self.toast("Revealed in Finder", theme::ACCENT);
            }

            // Settings
            if ui
                .add(IconButton {
                    icon: "settings",
                    active: self.settings_open,
                    tooltip: Some("Settings"),
                    ..Default::default()
                })
                .clicked()
            {
                self.settings_open = !self.settings_open;
            }

            // Right side
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.record_button(ui);
                if self.is_recording {
                    ui.add_space(10.0);
                    self.timer(ui);
                    ui.add_space(8.0);
                    self.pulse(ui);
                }
            });
        });
    }

    fn format_chip(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(78.0, 28.0), Sense::click());
        let p = ui.painter();
        p.rect_filled(rect, Rounding::same(8.0), theme::CARD);
        p.rect_stroke(rect, Rounding::same(8.0), theme::hair_stroke());

        let (label, color) = match self.cfg.format {
            Format::Mkv => ("MKV", theme::ACCENT),
            Format::Mp4 => ("MP4", theme::ORANGE),
        };
        p.circle_filled(Pos2::new(rect.left() + 13.0, rect.center().y), 3.5, color);
        p.text(
            Pos2::new(rect.left() + 24.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(12.0),
            theme::TEXT1,
        );

        let resp = resp.on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text("Recording format — click to switch. MKV is crash-safe; MP4 is for the final export.");
        if resp.clicked() {
            self.cfg.format = if self.cfg.format == Format::Mkv { Format::Mp4 } else { Format::Mkv };
        }
    }

    fn record_button(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(132.0, 38.0), Sense::click());
        let (fill, fg, glyph, label) = if self.is_recording {
            (Color32::from_rgb(0x1C, 0x1C, 0x1E), theme::RED, "stop", "Stop")
        } else {
            (theme::RED, Color32::WHITE, "record", "Record")
        };
        ui.painter().rect_filled(rect, Rounding::same(19.0), fill);
        // icon
        let _ = ui.put(
            Rect::from_center_size(Pos2::new(rect.left() + 22.0, rect.center().y), Vec2::splat(14.0)),
            icons::image(glyph, 14.0, fg),
        );
        ui.painter().text(
            Pos2::new(rect.left() + 40.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.5),
            fg,
        );

        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        if resp.clicked() {
            self.toggle_record();
        }
    }

    fn timer(&self, ui: &mut egui::Ui) {
        let secs = self.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        let text = if h > 0 {
            format!("{}:{:02}:{:02}", h, m, s)
        } else {
            format!("{:02}:{:02}", m, s)
        };
        ui.label(egui::RichText::new(text).monospace().size(13.0).color(theme::TEXT2));
    }

    fn pulse(&self, ui: &mut egui::Ui) {
        let t = ui.input(|i| i.time) as f32;
        let a = 0.4 + 0.6 * ((t * 3.0).sin() * 0.5 + 0.5);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 4.0, theme::RED.gamma_multiply(a));
        ui.ctx().request_repaint();
    }

    fn status_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            icons::draw(ui, "drive", 14.0, theme::TEXT3);
            ui.label(
                egui::RichText::new(format!("Saved to {}", self.cfg.output_path))
                    .size(11.0)
                    .color(theme::TEXT2),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some((_, gb, mins)) = self.disk {
                    ui.label(
                        egui::RichText::new(format!("{:.0} GB free · ~{} min remaining", gb, mins))
                            .size(11.0)
                            .color(theme::TEXT2),
                    );
                }
                ui.label(
                    egui::RichText::new(match self.cfg.mode {
                        Mode::Raw => "Raw capture · uncompressed feed",
                        Mode::Live => "Live zoom · double-tap ⇧ to follow the cursor",
                    })
                    .size(11.0)
                    .color(theme::TEXT3),
                );
            });
        });
    }

    fn settings(&mut self, ui: &mut egui::Ui, _progress: f32) {
        ui.horizontal_top(|ui| {
            // Sidebar
            ui.vertical(|ui| {
                ui.set_width(168.0);
                ui.set_min_height(EXPANDED - 24.0);
                for (i, tab) in panels::TABS.iter().enumerate() {
                    let selected = *tab == self.tab;
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
                    if selected {
                        ui.painter().rect_filled(rect, Rounding::same(8.0), theme::ACCENT);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, Rounding::same(8.0), theme::CONTROL);
                    }
                    let col = if selected { Color32::WHITE } else { theme::TEXT2 };
                    let _ = ui.put(
                        Rect::from_center_size(Pos2::new(rect.left() + 18.0, rect.center().y), Vec2::splat(15.0)),
                        icons::image(panels::TAB_ICONS[i], 15.0, col),
                    );
                    ui.painter().text(
                        Pos2::new(rect.left() + 34.0, rect.center().y - 6.0),
                        Align2::LEFT_CENTER,
                        panels::TAB_LABELS[i],
                        FontId::proportional(12.5),
                        if selected { Color32::WHITE } else { theme::TEXT1 },
                    );
                    ui.painter().text(
                        Pos2::new(rect.left() + 34.0, rect.center().y + 7.0),
                        Align2::LEFT_CENTER,
                        panels::TAB_BLURB[i],
                        FontId::proportional(9.5),
                        if selected { Color32::from_rgba_premultiplied(255,255,255,160) } else { theme::TEXT3 },
                    );
                    if resp.clicked() {
                        self.tab = *tab;
                    }
                    resp.on_hover_cursor(CursorIcon::PointingHand);
                }
            });

            ui.separator();

            // Content (scrollable field area with footer)
            ui.vertical(|ui| {
                ui.push_id("content", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(EXPANDED - 64.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            egui::Frame::none().fill(theme::FIELD).inner_margin(Margin::same(12.0)).show(ui, |ui| {
                                panels::show(ui, self.tab, &mut self.cfg, &mut self.listening);
                            });
                        });
                });
            });
        });
    }

    fn draw_toast(&self, ctx: &egui::Context) {
        if let Some((t, msg, color)) = &self.toast {
            if t.elapsed() > Duration::from_millis(2600) {
                return;
            }
            let screen = ctx.screen_rect();
            egui::Area::new(egui::Id::new("toast"))
                .order(egui::Order::Foreground)
                .fixed_pos(Pos2::new(screen.center().x - 150.0, screen.bottom() - 56.0))
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(28, 28, 32))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.set_width(300.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("●").color(*color).size(12.0));
                                ui.label(egui::RichText::new(msg).color(Color32::WHITE).size(12.5));
                            });
                        });
                });
        }
    }
}

// silence unused imports kept for the grouped helpers used by panels
#[allow(unused_imports)]
use crate::ui as _u;
