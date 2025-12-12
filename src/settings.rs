use serde::{Deserialize, Serialize};
use gpui::{px, WindowBounds, Bounds, Point, Size};
use std::path::PathBuf;
use std::fs;
use directories::ProjectDirs;
use tracing::warn;

/// Persisted app settings (font, theme, preferences).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    /// Font family name for the editor.
    pub font_family: String,
    /// Font size in pixels.
    pub font_size: f32,

    /// Name of the active theme.
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Whether to warn about unsaved changes.
    #[serde(default = "default_true")]
    pub enable_unsaved_changes_protection: bool,
}

fn default_true() -> bool { true }

fn default_theme() -> String {
    "Default Light".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            font_family: "Arial".to_string(),
            font_size: 14.0,
            theme: default_theme(),
            enable_unsaved_changes_protection: true,
        }
    }
}

/// Get the config directory, creating it if needed.
fn get_config_dir() -> PathBuf {
    let proj_dirs = ProjectDirs::from("com", "OneText", "OneText")
        .expect("Could not determine config directory for this platform");
    let config_dir = proj_dirs.config_dir().to_path_buf();
    if !config_dir.exists() {
        if let Err(e) = fs::create_dir_all(&config_dir) {
            warn!("Failed to create config directory: {}", e);
        }
    }
    config_dir
}

impl AppSettings {
    fn get_config_path() -> PathBuf {
        get_config_dir().join("settings.json")
    }

    /// Load from disk, or use defaults if missing.
    pub fn load() -> Self {
        if let Ok(contents) = fs::read_to_string(Self::get_config_path()) {
            if let Ok(settings) = serde_json::from_str(&contents) {
                return settings;
            }
        }
        Self::default()
    }

    /// Save to disk.
    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(Self::get_config_path(), json);
        }
    }

    pub fn window_bounds() -> WindowBounds {
        let state = WindowState::load();
        let width = if state.width > 0.0 { state.width } else { 800.0 };
        let height = if state.height > 0.0 { state.height } else { 600.0 };
        
        let size = Size { width: px(width), height: px(height) };
        if let (Some(x), Some(y)) = (state.x, state.y) {
            WindowBounds::Windowed(Bounds::new(Point { x: px(x), y: px(y) }, size))
        } else {
            // Fallback to fixed position when no saved position exists
            WindowBounds::Windowed(Bounds::new(Point { x: px(100.0), y: px(100.0) }, size))
        }
    }
}

/// Separate window state to avoid race condition with main settings.
/// Saved to a different file and only updated by the persistence thread.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WindowState {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: f32,
    pub height: f32,
}

impl WindowState {
    fn get_path() -> PathBuf {
        get_config_dir().join("window_state.json")
    }

    pub fn load() -> Self {
        if let Ok(contents) = fs::read_to_string(Self::get_path()) {
            if let Ok(state) = serde_json::from_str(&contents) {
                return state;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(Self::get_path(), json);
        }
    }
}