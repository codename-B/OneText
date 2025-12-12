#![windows_subsystem = "windows"]

mod settings;
mod workspace;
mod editor;

use gpui::*;
use gpui_component::{Root, Theme, ThemeRegistry};
use gpui_component::input::{Copy, Cut, Paste, SelectAll};
use gpui_component_assets::Assets;
use clap::Parser;
use std::path::PathBuf;
use tracing::warn;
use workspace::Workspace;
use settings::AppSettings;
use crate::editor::{UndoAction, RedoAction}; // Import editor actions

/// Returns the compilation directory or the directory containing the executable.
pub fn get_app_root() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.to_path_buf();
        }
    }
    PathBuf::from(".")
}

// Define Global Actions
actions!(global, [
    ExportPdfAction,
    NewFileAction,
    OpenFileDialogAction,
    SaveFileAction,
    SaveFileAsAction,
    FindAction,
    ExitAppAction
]);

#[derive(Parser, Debug)]
#[command(name = "OneText")]
#[command(version = "0.1.2")]
#[command(about = "A text editor", long_about = None)]
struct Cli {
    /// Optional file to open on startup
    file: Option<PathBuf>,
}

fn main() {
    // Initialize tracing for structured logging (only in debug builds by default)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into())
        )
        .init();

    let args = Cli::parse();
    let settings = AppSettings::load();

    let options = WindowOptions {
        window_bounds: Some(AppSettings::window_bounds()),
        titlebar: Some(gpui_component::TitleBar::title_bar_options()),
        ..Default::default()
    };

    Application::new().with_assets(Assets).run(move |cx: &mut App| {
        // Initialize gpui-component (required before using components)
        gpui_component::init(cx);

        // Load themes and set the default theme
        let theme_name = SharedString::from(settings.theme.clone());
        if let Err(err) = ThemeRegistry::watch_dir(
            get_app_root().join("assets/themes"),
            cx,
            move |cx| {
                if let Some(theme) = ThemeRegistry::global(cx)
                    .themes()
                    .get(&theme_name)
                    .cloned()
                {
                    Theme::global_mut(cx).apply_config(&theme);
                }
            }
        ) {
            warn!(error = %err, "Failed to watch themes directory");
        }

        // Global Keybindings
        cx.bind_keys([
            KeyBinding::new("ctrl-p", ExportPdfAction, None),
            KeyBinding::new("ctrl-f", FindAction, None),
            KeyBinding::new("ctrl-n", NewFileAction, None),
            KeyBinding::new("ctrl-o", OpenFileDialogAction, None),
            KeyBinding::new("ctrl-s", SaveFileAction, None),
            KeyBinding::new("ctrl-shift-s", SaveFileAsAction, None),
            KeyBinding::new("alt-f4", ExitAppAction, None),
            // editor bindings
            KeyBinding::new("ctrl-c", Copy, None),
            KeyBinding::new("ctrl-v", Paste, None),
            KeyBinding::new("ctrl-x", Cut, None),
            KeyBinding::new("ctrl-a", SelectAll, None),
            KeyBinding::new("ctrl-z", UndoAction, None),
            KeyBinding::new("ctrl-shift-z", RedoAction, None),
            KeyBinding::new("ctrl-y", RedoAction, None), // Alternate Redo
        ]);

        let file_to_open = args.file.clone();

        let window = cx.open_window(options, move |window, cx| {
            // Create the workspace view
            let workspace = cx.new(|cx| {
                let mut ws = Workspace::new(window, cx, settings.clone());
                if let Some(path) = file_to_open.clone() {
                    ws.open_file(path, window, cx);
                }
                ws
            });

            // Window Persistence Polling (Windows Only)
            #[cfg(target_os = "windows")]
            {
                std::thread::spawn(move || {
                    use windows::Win32::Foundation::{HWND, BOOL, LPARAM, RECT};
                    use windows::Win32::UI::WindowsAndMessaging::{
                        GetWindowRect, EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
                    };
                    use windows::Win32::System::Threading::GetCurrentProcessId;

                    let mut consecutive_failures = 0u32;

                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        
                        // Find window belonging to this process
                        let bounds_opt: Option<(f32, f32, f32, f32)> = unsafe {
                            struct FindData {
                                pid: u32,
                                hwnd: HWND,
                            }
                            
                            unsafe extern "system" fn enum_proc(window: HWND, param: LPARAM) -> BOOL {
                                let data = &mut *(param.0 as *mut FindData);
                                let mut pid = 0u32;
                                GetWindowThreadProcessId(window, Some(&mut pid));
                                if pid == data.pid && IsWindowVisible(window).as_bool() {
                                    data.hwnd = window;
                                    return BOOL(0); // Stop enumeration
                                }
                                BOOL(1) // Continue
                            }
                            
                            let pid = GetCurrentProcessId();
                            let mut data = FindData { pid, hwnd: HWND(0) };
                            let _ = EnumWindows(Some(enum_proc), LPARAM(&mut data as *mut _ as isize));
                            
                            if data.hwnd.0 != 0 {
                                let mut rect = RECT::default();
                                if GetWindowRect(data.hwnd, &mut rect).as_bool() {
                                    let w = (rect.right - rect.left) as f32;
                                    let h = (rect.bottom - rect.top) as f32;
                                    Some((rect.left as f32, rect.top as f32, w, h))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };

                        if let Some((x, y, w, h)) = bounds_opt {
                            consecutive_failures = 0; // Reset on success
                            
                            // Use separate WindowState to avoid race with main settings
                            let state = settings::WindowState::load();
                            let changed = state.x != Some(x) || state.y != Some(y) ||
                                          (state.width - w).abs() > 1.0 || (state.height - h).abs() > 1.0;
                            
                            if changed {
                                let new_state = settings::WindowState {
                                    x: Some(x),
                                    y: Some(y),
                                    width: w,
                                    height: h,
                                };
                                new_state.save();
                            }
                        } else {
                            // Window not found - app may be closing
                            consecutive_failures += 1;
                            if consecutive_failures >= 3 {
                                break; // Exit thread after 3 consecutive failures
                            }
                        }
                    }
                });
            }

            // Wrap in Root - this MUST be the top-level view in the window
            cx.new(|cx| Root::new(workspace.clone(), window, cx))
        }).expect("Failed to create main window");

        // Focus the workspace/editor after window is created
        window.update(cx, |_root, _window, cx| {
            // Root doesn't have focus_editor, so we need to access it through the workspace
            // For now, just activate the window
            cx.activate(true);
        }).ok();
    });
}
