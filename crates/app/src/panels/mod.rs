pub mod advanced;
pub mod audio;
pub mod general;
pub mod recording;
pub mod shortcuts;

use crate::config::AppConfig;
use crate::panels::shortcuts::HotkeyField;
use egui::Ui;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tab {
    Recording,
    Audio,
    General,
    Shortcuts,
    Advanced,
}

pub const TABS: [Tab; 5] = [
    Tab::Recording,
    Tab::Audio,
    Tab::General,
    Tab::Shortcuts,
    Tab::Advanced,
];
pub const TAB_LABELS: [&str; 5] = ["Recording", "Audio", "General", "Shortcuts", "Advanced"];
pub const TAB_ICONS: [&str; 5] = ["film", "waveform", "settings", "keyboard", "gauge"];
pub const TAB_BLURB: [&str; 5] = [
    "Format, resolution & quality",
    "Microphone & system sound",
    "Output, tray & launch",
    "Global hotkeys & gestures",
    "Encoder & pipeline",
];

pub fn show(ui: &mut Ui, tab: Tab, cfg: &mut AppConfig, listening: &mut Option<HotkeyField>) {
    match tab {
        Tab::Recording => recording::show(ui, cfg),
        Tab::Audio => audio::show(ui, cfg),
        Tab::General => general::show(ui, cfg),
        Tab::Shortcuts => shortcuts::show(ui, cfg, listening),
        Tab::Advanced => advanced::show(ui, cfg),
    }
}
