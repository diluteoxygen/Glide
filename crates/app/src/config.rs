//! Persisted settings. `#[serde(default)]` makes the file forward-compatible.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Mode { Raw, Live }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Format { Mkv, Mp4 }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Encoder {
    Auto,
    #[serde(rename = "VideoToolbox")]
    VideoToolbox,
    Nvenc,
    Qsv,
    Amf,
    Vaapi,
    X264,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub mode: Mode,
    pub format: Format,
    pub export_mp4_copy: bool,

    pub resolution: String,
    pub framerate: u32,
    pub quality: String,
    pub capture_cursor: bool,
    pub capture_clicks: bool,
    pub hardware_accel: bool,

    pub mic_source: String,
    pub mic_enabled: bool,
    pub system_audio: bool,

    pub output_path: String,
    pub show_hud: bool,
    pub launch_at_login: bool,
    pub minimize_to_tray: bool, // <-- the toggle the bug was about
    pub countdown: u32,
    pub post_summary: bool,

    pub hotkey_start_stop: String,
    pub hotkey_pause_resume: String,

    pub encoder: Encoder,
    pub ring_buffer: u32,
    pub zoom_intensity: u32,
    pub verbose_logging: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let out = dirs::video_dir().or_else(dirs::home_dir).unwrap_or_default().join("Glide");
        Self {
            mode: Mode::Live,
            format: Format::Mkv,
            export_mp4_copy: true,
            resolution: "1080p".into(),
            framerate: 60,
            quality: "Crisp".into(),
            capture_cursor: true,
            capture_clicks: true,
            hardware_accel: true,
            mic_source: "MacBook Pro Microphone".into(),
            mic_enabled: true,
            system_audio: true,
            output_path: out.to_string_lossy().into_owned(),
            show_hud: true,
            launch_at_login: false,
            minimize_to_tray: true,
            countdown: 3,
            post_summary: true,
            hotkey_start_stop: "⌃⇧R".into(),
            hotkey_pause_resume: "⌃⇧P".into(),
            encoder: Encoder::Auto,
            ring_buffer: 4,
            zoom_intensity: 65,
            verbose_logging: false,
        }
    }
}

impl AppConfig {
    pub fn path() -> std::path::PathBuf {
        dirs::config_dir().unwrap_or_default().join("glide").join("config.toml")
    }
    pub fn load() -> Self {
        match std::fs::read_to_string(Self::path()) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }
    pub fn save(&self) {
        if let Some(parent) = Self::path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(s) = toml::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), s);
        }
    }
}
