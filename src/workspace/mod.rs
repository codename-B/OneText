//! Workspace module - the main container for the OneText editor.
//!
//! This module is split into:
//! - `mod.rs` - Core Workspace struct and basic operations
//! - `file_ops.rs` - File dialog operations (open, save, save-as)
//! - `menu.rs` - Menu bar building

mod file_ops;
mod menu;

use gpui::*;
use gpui_component::{Theme, ThemeRegistry};

use gpui_component::TitleBar;
use std::path::PathBuf;

use crate::{DecreaseFontSizeAction, ExitAppAction, FindAction, GoToLineAction, IncreaseFontSizeAction, InsertDateTimeAction, NewFileAction, OpenFileDialogAction, ReplaceAction, ResetFontSizeAction, SaveFileAction, SaveFileAsAction, ToggleLineNumbersAction};
use tracing::debug;
use crate::editor::TextEditor;
use crate::settings::AppSettings;

/// Font size limits and step for zoom actions.
const MIN_FONT_SIZE: f32 = 8.0;
const MAX_FONT_SIZE: f32 = 72.0;
const FONT_SIZE_STEP: f32 = 1.0;
const DEFAULT_FONT_SIZE: f32 = 14.0;

/// Line height choices offered in the View menu (multiples of font size).
pub(crate) const LINE_HEIGHTS: [f32; 7] = [1.0, 1.2, 1.4, 1.5, 1.6, 1.8, 2.0];

/// Main workspace - holds the editor and current file state.
pub struct Workspace {
    /// The active view being displayed.
    pub active_view: AnyView,
    /// The text editor entity.
    pub editor_entity: Option<Entity<TextEditor>>,
    /// Path to the currently open file.
    pub current_file: Option<PathBuf>,
    /// Application settings.
    pub settings: AppSettings,
    /// Cached window title to avoid redundant updates.
    cached_title: String,
    /// Last known modification time of the open file on disk.
    pub(crate) disk_mtime: Option<std::time::SystemTime>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, settings: AppSettings) -> Self {
        let editor = cx.new(|cx| TextEditor::new(window, cx, "".into()));
        let font_size = settings.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        let font_family: SharedString = settings.font_family.clone().into();
        let line_height = settings.line_height.clamp(LINE_HEIGHTS[0], LINE_HEIGHTS[LINE_HEIGHTS.len() - 1]);
        let show_line_numbers = settings.show_line_numbers;
        editor.update(cx, |ed, cx| {
            ed.font_size = font_size;
            ed.font_family = font_family.clone();
            ed.line_height = line_height;
            ed.set_line_numbers(show_line_numbers, window, cx);
        });
        Self::apply_ui_font(font_family, cx);

        Self::spawn_disk_watcher(window, cx);

        Self {
            active_view: editor.clone().into(),
            editor_entity: Some(editor),
            current_file: None,
            settings,
            cached_title: String::new(),
            disk_mtime: None,
        }
    }

    /// Poll the open file's mtime and offer to reload when it changes on disk.
    fn spawn_disk_watcher(window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(2))
                        .await;

                    let Ok(info) = this.update(&mut cx, |ws, _cx| {
                        ws.current_file.clone().zip(ws.disk_mtime)
                    }) else {
                        break; // workspace dropped
                    };
                    let Some((path, known_mtime)) = info else {
                        continue;
                    };

                    let current_mtime = std::fs::metadata(&path)
                        .ok()
                        .and_then(|m| m.modified().ok());
                    let Some(current_mtime) = current_mtime else {
                        continue; // file missing/unreadable; leave state alone
                    };
                    if current_mtime <= known_mtime {
                        continue;
                    }

                    // Record the new mtime first so we don't prompt repeatedly.
                    let is_dirty = this
                        .update(&mut cx, |ws, cx| {
                            ws.disk_mtime = Some(current_mtime);
                            ws.editor_entity
                                .as_ref()
                                .map(|e| e.read(cx).is_dirty)
                                .unwrap_or(false)
                        })
                        .unwrap_or(false);

                    let filename = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("This file")
                        .to_string();
                    let description = if is_dirty {
                        format!(
                            "{} was changed outside OneText.\nReload it and lose your unsaved changes?",
                            filename
                        )
                    } else {
                        format!("{} was changed outside OneText. Reload it?", filename)
                    };

                    let result = rfd::AsyncMessageDialog::new()
                        .set_title("File Changed on Disk")
                        .set_description(description)
                        .set_buttons(rfd::MessageButtons::YesNo)
                        .show()
                        .await;

                    if result == rfd::MessageDialogResult::Yes {
                        let read_path = path.clone();
                        let content = cx
                            .background_spawn(async move { std::fs::read_to_string(&read_path).ok() })
                            .await;
                        if let Some(content) = content {
                            let _ = this.update_in(&mut cx, |ws, window, cx| {
                                if let Some(editor) = &ws.editor_entity {
                                    editor.update(cx, |ed, cx| {
                                        let _ = ed.open_file(path.clone(), window, cx, Some(content));
                                    });
                                }
                                ws.refresh_disk_mtime();
                                ws.update_title(window, cx);
                                cx.notify();
                            });
                        }
                    }
                }
            }
        })
        .detach();
    }

    /// Re-read the open file's mtime from disk.
    pub(crate) fn refresh_disk_mtime(&mut self) {
        self.disk_mtime = self
            .current_file
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .and_then(|m| m.modified().ok());
    }

    pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.editor_entity {
            editor.update(cx, |ed, cx| {
                let _ = ed.open_file(path.clone(), window, cx, None);
            });
        }
        self.current_file = Some(path.clone());
        self.settings.remember_file(&path);
        self.refresh_disk_mtime();
        self.update_title(window, cx);
        cx.notify();
    }

    /// Open a file after checking for unsaved changes.
    pub(crate) fn open_file_checked(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.handle_unsaved_changes(window, cx, move |this, window, cx| {
            this.open_file(path.clone(), window, cx);
        });
    }

    pub(crate) fn toggle_restore_last_file(&mut self, cx: &mut Context<Self>) {
        self.settings.restore_last_file = !self.settings.restore_last_file;
        self.settings.save();
        cx.notify();
    }

    /// Flip the line-number gutter, persist the preference and apply it to the editor.
    pub(crate) fn toggle_line_numbers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings.show_line_numbers = !self.settings.show_line_numbers;
        self.settings.save();
        let show = self.settings.show_line_numbers;
        self.with_editor(cx, |ed, cx| ed.set_line_numbers(show, window, cx));
        cx.notify();
    }

    pub(crate) fn clear_recent_files(&mut self, cx: &mut Context<Self>) {
        self.settings.recent_files.clear();
        self.settings.save();
        cx.notify();
    }

    /// Build window title (filename + dirty marker).
    fn get_title_text(&self, cx: &Context<Self>) -> String {
        let filename = self.current_file.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("OneText");
            
        let is_dirty = self.editor_entity.as_ref()
            .map(|e| e.read(cx).is_dirty)
            .unwrap_or(false);
        
        if is_dirty {
            format!("{} *", filename)
        } else {
            filename.to_string()
        }
    }

    /// Sync window title with current state (only if changed).
    pub(crate) fn update_title(&mut self, window: &mut Window, cx: &Context<Self>) {
        let title = self.get_title_text(cx);
        if title != self.cached_title {
            debug!(title = title, "Updating window title");
            window.set_window_title(&title);
            self.cached_title = title;
        }
    }

    pub fn close_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.editor_entity {
            editor.update(cx, |ed, cx| ed.close_file(window, cx));
        }
        self.current_file = None;
        self.disk_mtime = None;
        self.update_title(window, cx);
        cx.notify();
    }

    pub fn new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.handle_unsaved_changes(window, cx, |this, window, cx| {
            this.close_file(window, cx);
        });
    }

    pub fn exit_app(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.handle_unsaved_changes(window, cx, |_this, _window, cx| {
            cx.quit();
        });
    }

    pub fn open_license(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let license_path = crate::get_app_root().join("assets").join("License.txt");
        self.open_file(license_path, window, cx);
    }

    // --- Editor Access ---

    /// Run closure on editor if present.
    pub fn with_editor<F, R>(&self, cx: &mut Context<Self>, f: F) -> Option<R>
    where
        F: FnOnce(&mut crate::editor::TextEditor, &mut Context<crate::editor::TextEditor>) -> R,
    {
        self.editor_entity.as_ref().map(|editor| editor.update(cx, f))
    }

    /// Set editor font size (clamped) and save preference.
    pub(crate) fn set_font_size(&mut self, size: f32, cx: &mut Context<Self>) {
        let size = size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        if (size - self.settings.font_size).abs() < f32::EPSILON {
            return;
        }
        self.settings.font_size = size;
        AppSettings::save(&self.settings);
        self.with_editor(cx, |ed, cx| {
            ed.font_size = size;
            cx.notify();
        });
        cx.notify();
    }

    /// Set the font family for the editor and the rest of the window, and save it.
    pub(crate) fn set_font_family(&mut self, family: String, cx: &mut Context<Self>) {
        if family == self.settings.font_family {
            return;
        }
        self.settings.font_family = family.clone();
        AppSettings::save(&self.settings);
        let family: SharedString = family.into();
        self.with_editor(cx, |ed, cx| {
            ed.font_family = family.clone();
            cx.notify();
        });
        Self::apply_ui_font(family, cx);
        cx.notify();
    }

    /// Make the whole window (menus, title bar, status bar, popups) use the
    /// chosen family. The component theme's font is what the root applies,
    /// and Theme::update refreshes every window.
    fn apply_ui_font(family: SharedString, cx: &mut App) {
        if Theme::global(cx).font_family != family {
            Theme::update(cx, |t| t.font_family = family);
        }
    }

    /// Set editor line height (multiple of font size) and save preference.
    pub(crate) fn set_line_height(&mut self, line_height: f32, cx: &mut Context<Self>) {
        let line_height = line_height.clamp(LINE_HEIGHTS[0], LINE_HEIGHTS[LINE_HEIGHTS.len() - 1]);
        if (line_height - self.settings.line_height).abs() < f32::EPSILON {
            return;
        }
        self.settings.line_height = line_height;
        AppSettings::save(&self.settings);
        self.with_editor(cx, |ed, cx| {
            ed.line_height = line_height;
            cx.notify();
        });
        cx.notify();
    }

    pub(crate) fn increase_font_size(&mut self, cx: &mut Context<Self>) {
        self.set_font_size(self.settings.font_size + FONT_SIZE_STEP, cx);
    }

    pub(crate) fn decrease_font_size(&mut self, cx: &mut Context<Self>) {
        self.set_font_size(self.settings.font_size - FONT_SIZE_STEP, cx);
    }

    pub(crate) fn reset_font_size(&mut self, cx: &mut Context<Self>) {
        self.set_font_size(DEFAULT_FONT_SIZE, cx);
    }

    /// Apply theme and save preference.
    pub(crate) fn apply_theme(&mut self, theme_name: String, cx: &mut Context<Self>) {
        let name = SharedString::from(theme_name);
        if let Some(theme) = ThemeRegistry::global(cx).themes().get(&name).cloned() {
            // Theme::update re-projects the colors onto the base layer that
            // paints the scrollbar and refreshes every window; writing through
            // global_mut would leave the scrollbar on the old palette.
            // Re-apply the chosen font after the theme file installs its own.
            let family: SharedString = self.settings.font_family.clone().into();
            Theme::update(cx, |t| {
                t.apply_config(&theme);
                t.font_family = family;
            });
            self.settings.theme = name.to_string();
            AppSettings::save(&self.settings);
        }
    }
}

// --- Render ---

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.update_title(window, cx);
        let theme = Theme::global_mut(cx);
        let palette = theme.colors;

        let menu_bar = self.build_menu_bar(window, cx);

        div()
            .id("workspace")
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(palette.background)
            .on_action(cx.listener(|this, _: &NewFileAction, window, cx| this.new_file(window, cx)))
            .on_action(cx.listener(|this, _: &OpenFileDialogAction, window, cx| this.open_dialog(window, cx)))
            .on_action(cx.listener(|this, _: &SaveFileAction, window, cx| this.save_file(window, cx)))
            .on_action(cx.listener(|this, _: &SaveFileAsAction, window, cx| this.save_as_dialog(window, cx)))
            .on_action(cx.listener(|this, _: &FindAction, window, cx| { this.with_editor(cx, |ed, cx| ed.open_search(window, cx)); }))
            .on_action(cx.listener(|this, _: &ReplaceAction, window, cx| { this.with_editor(cx, |ed, cx| ed.open_replace(window, cx)); }))
            .on_action(cx.listener(|this, _: &GoToLineAction, window, cx| { this.with_editor(cx, |ed, cx| ed.open_goto_line(window, cx)); }))
            .on_action(cx.listener(|this, _: &InsertDateTimeAction, window, cx| { this.with_editor(cx, |ed, cx| ed.insert_datetime(window, cx)); }))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                if let Some(path) = paths.paths().first().cloned() {
                    if path.is_file() {
                        this.open_file_checked(path, window, cx);
                    }
                }
            }))
            .on_action(cx.listener(|this, _: &ExitAppAction, window, cx| this.exit_app(window, cx)))
            .on_action(cx.listener(|this, _: &IncreaseFontSizeAction, _window, cx| this.increase_font_size(cx)))
            .on_action(cx.listener(|this, _: &DecreaseFontSizeAction, _window, cx| this.decrease_font_size(cx)))
            .on_action(cx.listener(|this, _: &ResetFontSizeAction, _window, cx| this.reset_font_size(cx)))
            .on_action(cx.listener(|this, _: &ToggleLineNumbersAction, window, cx| this.toggle_line_numbers(window, cx)))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                if event.modifiers.control {
                    let dy = event.delta.pixel_delta(window.line_height()).y;
                    if dy > px(0.0) {
                        this.increase_font_size(cx);
                    } else if dy < px(0.0) {
                        this.decrease_font_size(cx);
                    }
                    cx.stop_propagation();
                }
            }))
            .child(TitleBar::new().child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size_full()
                            .child(
                                div()
                                    .text_color(palette.foreground)
                                    .text_sm()
                                    .child(self.get_title_text(cx))
                            )
                    ))
            .child(menu_bar)
            .child(self.active_view.clone())
    }
}
