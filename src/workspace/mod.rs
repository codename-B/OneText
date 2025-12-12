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

use crate::{ExitAppAction, FindAction, NewFileAction, OpenFileDialogAction, SaveFileAction, SaveFileAsAction};
use tracing::debug;
use crate::editor::TextEditor;
use crate::settings::AppSettings;

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
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, settings: AppSettings) -> Self {
        let editor = cx.new(|cx| TextEditor::new(window, cx, "".into()));

        Self {
            active_view: editor.clone().into(),
            editor_entity: Some(editor),
            current_file: None,
            settings,
        }
    }

    pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.editor_entity {
            editor.update(cx, |ed, cx| {
                let _ = ed.open_file(path.clone(), window, cx, None);
            });
        }
        self.current_file = Some(path);
        self.update_title(window, cx);
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

    /// Sync window title with current state.
    pub(crate) fn update_title(&self, window: &mut Window, cx: &Context<Self>) {
        let title = self.get_title_text(cx);
        debug!(title = title, "Updating window title");
        window.set_window_title(&title);
    }

    pub fn close_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &self.editor_entity {
            editor.update(cx, |ed, cx| ed.close_file(window, cx));
        }
        self.current_file = None;
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

    /// Apply theme and save preference.
    pub(crate) fn apply_theme(&mut self, theme_name: String, cx: &mut Context<Self>) {
        let name = SharedString::from(theme_name);
        if let Some(theme) = ThemeRegistry::global(cx).themes().get(&name).cloned() {
            Theme::global_mut(cx).apply_config(&theme);
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
            .on_action(cx.listener(|this, _: &ExitAppAction, window, cx| this.exit_app(window, cx)))
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
