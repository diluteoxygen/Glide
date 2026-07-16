//! The "Audio" tab content: Input Source dropdown + System section with
//! the "Record system audio" toggle. This is the one tab the reference
//! image fully specifies — General/Shortcuts/Advance are deliberately
//! left as stubs (see their files) rather than inventing content.

use crate::theme;
use crate::widgets::{dropdown, toggle_switch};
use egui::Ui;

/// Everything the audio panel needs from the caller. `input_devices` and
/// `selected_input` are placeholders — Claude Code should wire these to
/// the real WASAPI/PipeWire device enumeration (see the `// BACKEND HOOK`
/// comments below).
#[derive(Default)]
pub struct AudioPanelState {
    pub record_system_audio: bool,
    pub selected_mic: Option<String>,
    pub available_mics: Vec<String>,
}

impl AudioPanelState {
    pub fn new() -> Self {
        Self {
            record_system_audio: true,
            selected_mic: None,
            available_mics: capture_windows::audio::WasapiCapturer::enumerate_microphones().unwrap_or_else(|_| vec!["Default Microphone".to_string()]),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut AudioPanelState) {
    ui.vertical(|ui| {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Input Source")
                .size(13.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(6.0);

        let mics = if state.available_mics.is_empty() {
            vec!["Default Microphone".to_string()]
        } else {
            state.available_mics.clone()
        };
        let current = state.selected_mic.clone().unwrap_or_else(|| mics.first().cloned().unwrap_or_else(|| "Select Microphone...".to_string()));
        if let Some(idx) = dropdown::show(ui, "mic_select", &current, &mics) {
            state.selected_mic = Some(mics[idx].clone());
            // BACKEND HOOK: propagate the new selected input device to the
            // audio capture backend here (or have the caller observe the
            // state change after `show()` returns — either is fine, just
            // don't leave this silently unwired).
        }

        ui.add_space(18.0);
        ui.label(
            egui::RichText::new("System")
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
                        ui.label(
                            egui::RichText::new("Record system audio")
                                .size(13.0)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        );
                        ui.label(
                            egui::RichText::new("Capture sound playing on this device")
                                .size(11.0)
                                .color(theme::TEXT_SECONDARY),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if toggle_switch::show(ui, "audio_record_system_audio", &mut state.record_system_audio) {
                            // BACKEND HOOK: enable/disable system-loopback
                            // capture (Phase 2's WASAPI loopback / PipeWire
                            // sink) in response to this toggle.
                        }
                    });
                });
            });
    });
}
