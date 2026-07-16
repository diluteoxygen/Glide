use crate::panels::advance_panel::AdvancePanelState;
use crate::panels::audio_panel::AudioPanelState;
use crate::panels::general_panel::GeneralPanelState;
use crate::panels::shortcuts_panel::ShortcutsPanelState;
use crate::panels::{advance_panel, audio_panel, general_panel, shortcuts_panel};
use crate::theme;
use crate::widgets::icons;
use crate::widgets::segmented_control::{self, RecordMode};
use crate::widgets::sidebar_tabs::{self, SettingsTab};
use eframe::egui;
use std::time::{Duration, Instant};

const COLLAPSED_HEIGHT: f32 = 96.0;
const EXPANDED_EXTRA_HEIGHT: f32 = 340.0;
const DEFAULT_WIDTH: f32 = 600.0;

pub struct GlideApp {
    mode: RecordMode,
    settings_open: bool,
    selected_tab: SettingsTab,

    audio_state: AudioPanelState,
    general_state: GeneralPanelState,
    shortcuts_state: ShortcutsPanelState,
    advance_state: AdvancePanelState,

    // BACKEND HOOK: replace with the real "is a recording in progress"
    // flag from the pipeline (the same one the HUD reads) — this local
    // bool is a stand-in until that's wired up.
    is_recording: bool,
    recording_started_at: Option<Instant>,

    // Tracks the last size we ourselves requested, so we only issue a new
    // ViewportCommand::InnerSize when something actually changed — sending
    // it unconditionally every frame is what caused the maximize bug
    // (see `apply_window_sizing` below for the full explanation).
    last_sent_size: Option<egui::Vec2>,
    toasts: egui_toast::Toasts,
    tray_icon: Option<tray_icon::TrayIcon>,
}

impl Default for GlideApp {
    fn default() -> Self {
        Self {
            mode: RecordMode::Raw,
            settings_open: false,
            selected_tab: SettingsTab::Audio,
            audio_state: AudioPanelState::new(),
            general_state: GeneralPanelState::default(),
            shortcuts_state: ShortcutsPanelState::default(),
            advance_state: AdvancePanelState::default(),
            is_recording: false,
            recording_started_at: None,
            last_sent_size: None,
            toasts: egui_toast::Toasts::new()
                .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),
            tray_icon: None,
        }
    }
}

impl eframe::App for GlideApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        theme::apply(&ctx);

        if self.is_recording {
            // 1s granularity is all the elapsed-time display needs; no
            // reason to repaint at full framerate just for the timer.
            ctx.request_repaint_after(Duration::from_millis(250));
        }

        let anim_id = egui::Id::new("settings_expand_progress");
        let progress = ctx.animate_bool(anim_id, self.settings_open);
        if progress > 0.0001 && progress < 0.9999 {
            ctx.request_repaint();
        }
        let target_height = COLLAPSED_HEIGHT + EXPANDED_EXTRA_HEIGHT * progress;

        // Handle Tray Icon Events
        if let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
            if let tray_icon::TrayIconEvent::Click { button: tray_icon::MouseButton::Left, button_state: tray_icon::MouseButtonState::Up, .. } = event {
                // Stop recording and restore window
                self.is_recording = false;
                self.recording_started_at = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                self.tray_icon = None; // Remove tray icon
                
                self.toasts.add(egui_toast::Toast {
                    text: "Recording Saved".into(),
                    kind: egui_toast::ToastKind::Success,
                    options: egui_toast::ToastOptions::default()
                        .duration_in_seconds(3.0)
                        .show_progress(true),
                    style: Default::default(),
                });
            }
        }

        self.apply_window_sizing(&ctx, progress, target_height);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::BG_WINDOW).inner_margin(egui::vec2(16.0, 12.0)))
            .show_inside(ui, |ui| {
                self.toolbar_row(ui);
                ui.add_space(8.0);
                self.info_row(ui, progress);

                if progress > 0.01 {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    let clip_height = (EXPANDED_EXTRA_HEIGHT * progress - 40.0).max(0.0);
                    // Fade the panel in with the same progress value so it
                    // doesn't just hard-cut into view.
                    let alpha = (progress * 255.0) as u8;
                    ui.scope(|ui| {
                        ui.set_max_height(clip_height);
                        ui.set_clip_rect(ui.max_rect());
                        ui.visuals_mut().override_text_color =
                            Some(theme::TEXT_PRIMARY.gamma_multiply(progress));
                        let _ = alpha; // kept for clarity if you want a hard alpha fade instead
                        self.settings_panel(ui);
                    });
                }
            });
        
        self.toasts.show(ui);
    }
}

impl GlideApp {
    /// Window-sizing logic, fixed to stop fighting the OS's own
    /// maximize/restore handling.
    ///
    /// THE BUG: this previously called
    /// `ctx.send_viewport_cmd(ViewportCommand::InnerSize(...))`
    /// unconditionally, every single frame, using a height computed from
    /// our own collapsed/expanded formula. When the user clicked native
    /// "maximize," the OS resized the window to full-screen dimensions —
    /// and then, on the very next frame, this code immediately forced the
    /// height back down to `target_height`, while leaving the
    /// OS-maximized width alone (since we read current width from the
    /// just-maximized window). That produced exactly the symptom
    /// reported: full-width, fixed/small height.
    ///
    /// THE FIX: don't touch size at all while the OS reports the window
    /// as maximized, and only send a resize command when the desired size
    /// actually changed (avoids redundant resize churn/flicker on every
    /// frame even in the normal, non-maximized case).
    fn apply_window_sizing(&mut self, ctx: &egui::Context, progress: f32, target_height: f32) {
        let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
        if maximized {
            self.last_sent_size = None;
            return;
        }

        let current_width = ctx
            .input(|i| i.viewport().inner_rect.map(|r| r.width()))
            .unwrap_or(DEFAULT_WIDTH);
        let desired = egui::vec2(current_width, target_height);

        let is_animating = progress > 0.0001 && progress < 0.9999;
        if !is_animating {
            // Force the exact final size once when the animation completes,
            // then stop sending InnerSize so the user can resize freely.
            if let Some(last) = self.last_sent_size {
                if (last - desired).length() > 0.5 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(desired));
                }
            }
            self.last_sent_size = None;
            return;
        }

        let changed = match self.last_sent_size {
            Some(last) => (last - desired).length() > 0.5,
            None => true,
        };
        if changed {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(desired));
            self.last_sent_size = Some(desired);
        }
    }

    fn toolbar_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(new_mode) = segmented_control::show(ui, self.mode) {
                self.mode = new_mode;
                // BACKEND HOOK: propagate Raw vs. Live-zoom(OTF) mode
                // choice to the recording session config here.
            }

            ui.add_space(10.0);

            ui.add_space(10.0);

            if icons::draw(ui, icons::FOLDER, 18.0, theme::TEXT_SECONDARY)
                .interact(egui::Sense::click())
                .clicked()
            {
                // BACKEND HOOK: open the output-directory picker.
            }
            ui.add_space(6.0);
            if icons::draw(ui, icons::MICROPHONE, 18.0, theme::TEXT_SECONDARY)
                .interact(egui::Sense::click())
                .clicked()
            {
                // BACKEND HOOK: quick-toggle mic capture on/off.
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.record_button(ui);
                if self.is_recording {
                    ui.add_space(10.0);
                    self.elapsed_timer(ui);
                }
            });
        });
    }

    /// Start/Stop are now visually distinct beyond the label text: fill
    /// color, icon, and shape all change, plus a pulsing dot appears while
    /// recording — a text-only swap was flagged as too easy to miss.
    fn record_button(&mut self, ui: &mut egui::Ui) {
        if self.is_recording {
            self.pulsing_dot(ui);
            ui.add_space(6.0);
        }

        let (fill, text_color, glyph, label) = if self.is_recording {
            (theme::TEXT_PRIMARY, theme::RECORD_RED, crate::widgets::icons::STOP, "Stop Recording")
        } else {
            (theme::RECORD_RED, theme::RECORD_RED_TEXT, crate::widgets::icons::RECORD, "Start Recording")
        };

        let button = egui::Button::new(
            egui::RichText::new(format!("{glyph}  {label}")).color(text_color).strong(),
        )
        .fill(fill)
        .corner_radius(theme::rounding_small())
        .min_size(egui::vec2(150.0, 34.0));

        let ctx = ui.ctx().clone();
        if ui.add(button).clicked() {
            self.is_recording = !self.is_recording;
            if self.is_recording {
                self.recording_started_at = Some(std::time::Instant::now());
                
                // Minimize to tray
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                
                // Create Tray Icon
                let rgba = vec![255; 4 * 16 * 16];
                if let Ok(icon) = tray_icon::Icon::from_rgba(rgba, 16, 16) {
                    self.tray_icon = tray_icon::TrayIconBuilder::new()
                        .with_tooltip("Glide is Recording (Click to stop)")
                        .with_icon(icon)
                        .build()
                        .ok();
                }

                if self.settings_open {
                    self.settings_open = false;
                }
            } else {
                self.recording_started_at = None;
                self.toasts.add(egui_toast::Toast {
                    text: "Recording Saved".into(),
                    kind: egui_toast::ToastKind::Success,
                    options: egui_toast::ToastOptions::default()
                        .duration_in_seconds(3.0)
                        .show_progress(true),
                    style: Default::default(),
                });
            }
        }
    }

    fn pulsing_dot(&self, ui: &mut egui::Ui) {
        let t = ui.input(|i| i.time) as f32;
        let alpha = 0.4 + 0.6 * ((t * 3.0).sin() * 0.5 + 0.5);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 4.0, theme::RECORD_RED.gamma_multiply(alpha));
        ui.ctx().request_repaint(); // smooth breathing needs per-frame updates while active
    }

    fn elapsed_timer(&self, ui: &mut egui::Ui) {
        if let Some(started) = self.recording_started_at {
            let secs = started.elapsed().as_secs();
            let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
            let text = if h > 0 {
                format!("{h:02}:{m:02}:{s:02}")
            } else {
                format!("{m:02}:{s:02}")
            };
            ui.label(
                egui::RichText::new(text)
                    .monospace()
                    .size(13.0)
                    .color(theme::TEXT_SECONDARY),
            );
        }
    }

    fn info_row(&mut self, ui: &mut egui::Ui, expand_progress: f32) {
        ui.horizontal(|ui| {
            icons::draw(ui, icons::INFO, 14.0, theme::TEXT_SECONDARY);
            ui.add_space(4.0);

            let hint = match self.mode {
                RecordMode::Raw => "Raw capture mode: uncompressed feed.",
                // BACKEND HOOK: pull this from the OTF gesture spec if it
                // ever changes, rather than duplicating the string here
                // and in docs/otf/ARCHITECTURE.md.
                RecordMode::Live => "Live zoom: double-tap Shift to follow the cursor.",
            };
            ui.label(egui::RichText::new(hint).size(12.0).color(theme::TEXT_SECONDARY));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Chevron rotates smoothly between down/up instead of
                // snapping instantly, using the same expand_progress the
                // panel height animates with.
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::click());
                let angle = std::f32::consts::PI * expand_progress; // 0 = down, PI = up
                draw_rotating_chevron(ui.painter(), rect, theme::TEXT_SECONDARY, angle);

                ui.add_space(4.0);
                let label_resp =
                    ui.label(egui::RichText::new("More Settings").size(12.0).color(theme::TEXT_SECONDARY));

                if response.clicked() || label_resp.clicked() {
                    self.settings_open = !self.settings_open;
                }
            });
        });
    }

    fn settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_top(|ui| {
            if let Some(new_tab) = sidebar_tabs::show(ui, self.selected_tab) {
                self.selected_tab = new_tab;
            }

            ui.separator();

            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width());
                match self.selected_tab {
                    SettingsTab::General => general_panel::show(ui, &mut self.general_state),
                    SettingsTab::Audio => audio_panel::show(ui, &mut self.audio_state),
                    SettingsTab::Shortcuts => shortcuts_panel::show(ui, &mut self.shortcuts_state),
                    SettingsTab::Advance => advance_panel::show(ui, &mut self.advance_state),
                }
            });
        });
    }
}

/// Draws a chevron that smoothly interpolates its point direction between
/// down (angle=0) and up (angle=PI), instead of the old implementation
/// which just picked one of two static shapes.
fn draw_rotating_chevron(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32, angle: f32) {
    let cx = rect.center().x;
    let cy = rect.center().y;
    let half_w = rect.width() * 0.3;
    let half_h = rect.height() * 0.3;

    // Base "pointing down" shape, then rotate every point by `angle`
    // around the rect's center.
    let base = [
        egui::vec2(-half_w, -half_h * 0.3),
        egui::vec2(0.0, half_h),
        egui::vec2(half_w, -half_h * 0.3),
    ];
    let (sin_a, cos_a) = angle.sin_cos();
    let rotated: Vec<egui::Pos2> = base
        .iter()
        .map(|v| egui::pos2(cx + v.x * cos_a - v.y * sin_a, cy + v.x * sin_a + v.y * cos_a))
        .collect();

    let stroke = egui::Stroke::new(1.6_f32, color);
    painter.line_segment([rotated[0], rotated[1]], stroke);
    painter.line_segment([rotated[1], rotated[2]], stroke);
}
