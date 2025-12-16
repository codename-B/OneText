use gpui::*;
use gpui_component::{
    Theme, input::{
        Copy as CopyAction,
        Cut as CutAction,
        Input,
        InputEvent,
        InputState,
        Paste as PasteAction,
        Search as SearchAction,
        SelectAll as SelectAllAction,
        Position,
    }
};
use std::path::PathBuf;
use tracing::{debug, warn, info};
use crate::ExportPdfAction;

mod fps;
mod pdf;
mod types;

pub use fps::FpsTracker;
pub use types::{LineEnding, Encoding};

mod history;
use history::History;

// Actions
actions!(editor, [UndoAction, RedoAction, NormalizePasteAction]);

/// Main text editor component with multi-line input, undo/redo, and status bar.
pub struct TextEditor {
    /// The underlying input state entity.
    pub(crate) input_state: Entity<InputState>,
    /// Path to the currently open file, if any.
    pub(crate) current_file: Option<PathBuf>,
    encoding: Encoding,
    line_ending: LineEnding,
    /// Whether soft wrap is enabled.
    pub(crate) soft_wrap: bool,
    /// Whether the content allows edits.
    #[allow(dead_code)]
    pub read_only: bool,
    /// Whether the content has unsaved changes.
    pub is_dirty: bool,
    /// Whether to ignore input events (e.g. during file load).
    ignore_input_events: bool,
    /// Whether the status bar is visible.
    pub(crate) show_status_bar: bool,
    fps_tracker: FpsTracker,
    history: History,
    _subscriptions: Vec<Subscription>,
}

impl TextEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, initial_text: String) -> Self {
        // Create InputState with multi-line support
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .searchable(true)
                .soft_wrap(true)
        });

        // Set initial text if provided
        if !initial_text.is_empty() {
            input_state.update(cx, |state, cx| {
                state.set_value(&initial_text, window, cx);
            });
        }

        // Subscribe to input events
        let _subscriptions = vec![
            cx.subscribe_in(&input_state, window, {
                move |this, _, _ev: &InputEvent, _window, cx| {
                    if !this.ignore_input_events {
                        // Capture snapshot
                        let state = this.input_state.read(cx);
                        let text = state.value().to_string();
                        let cursor = state.cursor();
                        
                        this.history.push(text, cursor, cursor);
                        this.update_dirty_state(cx);
                    }
                    cx.notify();
                }
            })
        ];

        Self {
            input_state,
            current_file: None,
            encoding: Encoding::default(),
            line_ending: LineEnding::default(),
            soft_wrap: true,
            read_only: false,
            is_dirty: false,
            ignore_input_events: false,
            show_status_bar: true,
            fps_tracker: FpsTracker::new(),
            history: History::new(),
            _subscriptions,
        }
    }

    pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>, content: Option<String>) -> anyhow::Result<()> {
        let content = match content {
            Some(c) => c,
            None => std::fs::read_to_string(&path)?,
        };
        let content = normalize_tabs(&content);

        self.ignore_input_events = true;
        self.input_state.update(cx, |state, cx| {
            state.set_value(&content, window, cx);
        });
        
        // Reset ignore flag on next frame strictly to catch deferred events
        cx.on_next_frame(window, |this: &mut Self, _window: &mut Window, _cx| {
            this.ignore_input_events = false;
        });

        self.current_file = Some(path);
        self.line_ending = LineEnding::detect(&content);
        self.encoding = Encoding::default();
        
        self.history.clear(content);
        self.update_dirty_state(cx);
        
        cx.notify();
        Ok(())
    }

    /// Mark as saved (clears dirty flag).
    pub fn mark_clean(&mut self) {
        self.history.mark_saved();
        self.is_dirty = false;
    }

    pub fn close_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Clear the editor content
        self.ignore_input_events = true;
        self.input_state.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        
        // Reset ignore flag on next frame
        cx.on_next_frame(window, |this: &mut Self, _window: &mut Window, _cx| {
            this.ignore_input_events = false;
        });

        // Clear current file reference
        self.current_file = None;
        self.line_ending = LineEnding::default();
        self.encoding = Encoding::default();
        
        self.history.clear(String::new());
        self.update_dirty_state(cx);
        
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn set_text(&mut self, content: String, window: &mut Window, cx: &mut Context<Self>) {
        debug!(
            len = content.len(),
            path = ?self.current_file,
            "Setting editor text"
        );
        self.input_state.update(cx, |state, cx| {
            state.set_value(&content, window, cx);
        });
        self.line_ending = LineEnding::detect(&content);
        self.encoding = Encoding::default();
        cx.notify();
    }

    // --- Input Actions ---
    // Focus the input and dispatch an action to it.

    pub fn copy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_to_input(&CopyAction, window, cx);
    }

    pub fn cut(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_to_input(&CutAction, window, cx);
    }

    pub fn paste(&mut self, _: &NormalizePasteAction, window: &mut Window, cx: &mut Context<Self>) {
        // Normalize tabs in clipboard content before pasting
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                let normalized = normalize_tabs(&text);
                cx.write_to_clipboard(ClipboardItem::new_string(normalized));
            }
        }
        self.dispatch_to_input(&PasteAction, window, cx);
    }

    pub fn select_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_to_input(&SelectAllAction, window, cx);
    }

    pub fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dispatch_to_input(&SearchAction, window, cx);
    }

    /// Focus input and dispatch action.
    fn dispatch_to_input(&self, action: &dyn Action, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.focus_handle(cx);
        focus.focus(window);
        focus.dispatch_action(action, window, cx);
    }

    pub fn toggle_soft_wrap(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.soft_wrap = !self.soft_wrap;
        self.input_state.update(cx, |state, cx| {
            state.set_soft_wrap(self.soft_wrap, window, cx);
        });
        cx.notify();
    }

    pub fn toggle_status_bar(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_status_bar = !self.show_status_bar;
        cx.notify();
    }

    pub fn undo(&mut self, _: &UndoAction, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(snapshot) = self.history.undo() {
            let text = snapshot.text.clone();
            // Ignore input events while restoring state
            self.ignore_input_events = true;
            self.input_state.update(cx, |state, cx| {
                state.set_value(&text, window, cx);
                let pos = Self::offset_to_position(&text, snapshot.cursor_head);
                state.set_cursor_position(pos, window, cx);
            });
            cx.on_next_frame(window, |this: &mut Self, _window, _cx| {
                this.ignore_input_events = false;
            });
            self.update_dirty_state(cx);
        }
    }

    fn offset_to_position(text: &str, offset: usize) -> Position {
        let mut line = 0;
        let mut character = 0;
        
        for (i, c) in text.char_indices() {
            if i >= offset { break; }
            if c == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }
        Position { line, character }
    }

    pub fn redo(&mut self, _: &RedoAction, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(snapshot) = self.history.redo() {
            let text = snapshot.text.clone();
            self.ignore_input_events = true;
            self.input_state.update(cx, |state, cx| {
                state.set_value(&text, window, cx);
                let pos = Self::offset_to_position(&text, snapshot.cursor_head);
                state.set_cursor_position(pos, window, cx);
            });
            cx.on_next_frame(window, |this: &mut Self, _window, _cx| {
                this.ignore_input_events = false;
            });
            self.update_dirty_state(cx);
        }
    }

    fn update_dirty_state(&mut self, cx: &mut Context<Self>) {
        let dirty = self.history.is_dirty();
        if self.is_dirty != dirty {
            self.is_dirty = dirty;
            cx.notify();
        }
    }

    /// Export to PDF via save dialog.
    pub fn export_pdf(&mut self, _: &ExportPdfAction, window: &mut Window, cx: &mut Context<Self>) {
        let content = self.input_state.read(cx).value().to_string();
        let filename = self.current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();
        
        // Get theme colors for PDF
        let theme = Theme::global(cx);
        let bg = theme.colors.background;
        let fg = theme.colors.foreground;
        
        // Convert HSLA to RGB (0-255)
        let bg_rgb = hsla_to_rgb_u8(bg);
        let fg_rgb = hsla_to_rgb_u8(fg);
        
        let config = pdf::PdfConfig {
            font_size: 12.0,
            margin: 72.0, // 1 inch in points
            header: Some(format!("{} - {}", filename, current_date())),
            background_rgb: bg_rgb,
            text_rgb: fg_rgb,
        };
        
        // Spawn async task to show save dialog and export
        cx.spawn_in(window, move |_this, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let dialog_task = cx.background_spawn(async move {
                    let mut path = PathBuf::from(&filename);
                    path.set_extension("pdf");
                    rfd::AsyncFileDialog::new()
                        .add_filter("PDF", &["pdf"])
                        .set_file_name(path.file_name().unwrap().to_str().unwrap())
                        .save_file()
                        .await
                        .map(|f| f.path().to_path_buf())
                });
                
                if let Some(path) = dialog_task.await {
                    info!(path = ?path, "Exporting to PDF");
                    match pdf::export_to_pdf(&content, &path, &config) {
                        Ok(_) => info!("PDF export completed"),
                        Err(e) => warn!(error = %e, "PDF export failed"),
                    }
                }
                let _ = cx.update(|_, _| {});
            }
        })
        .detach();
    }
}

/// HSLA to RGB (0-255).
fn hsla_to_rgb_u8(hsla: Hsla) -> (u8, u8, u8) {
    let h = hsla.h;
    let s = hsla.s;
    let l = hsla.l;
    
    let (r, g, b) = if s == 0.0 {
        (l, l, l)
    } else {
        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;
        
        let hue_to_rgb = |p: f32, q: f32, mut t: f32| -> f32 {
            if t < 0.0 { t += 1.0; }
            if t > 1.0 { t -= 1.0; }
            if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
            if t < 1.0 / 2.0 { return q; }
            if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
            p
        };
        
        (
            hue_to_rgb(p, q, h + 1.0 / 3.0),
            hue_to_rgb(p, q, h),
            hue_to_rgb(p, q, h - 1.0 / 3.0),
        )
    };
    
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// Current date as YYYY-MM-DD.
fn current_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

impl Focusable for TextEditor {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input_state.read(cx).focus_handle(cx)
    }
}

impl Render for TextEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Only request continuous animation frames when status bar with FPS is visible
        if self.show_status_bar {
            window.request_animation_frame();
        }

        // Calculate FPS using the tracker
        let fps = self.fps_tracker.tick().round() as u32;

        let theme = Theme::global_mut(cx);
        let colors = theme.colors;
        let cursor = self.input_state.read(cx).cursor_position();
        let line = cursor.line.saturating_add(1);
        let column = cursor.character.saturating_add(1);
        let char_count = self.input_state.read(cx).value().chars().count();
        let char_count_display = Self::format_with_commas(char_count);
        let selected_text_range = self.input_state.update(cx, |state, cx| {
            state.selected_text_range(true, window, cx)
        });

        // Calculate selection length from UTF-16 indices
        let selection_len = selected_text_range.as_ref()
            .map(|selection| {
                let start = selection.range.start.min(selection.range.end);
                let end = selection.range.start.max(selection.range.end);
                end - start
            })
            .unwrap_or(0);

        // Show selection count if text is selected, otherwise show total char count
        let count_display = if selection_len > 0 {
            format!("{} of {} characters", Self::format_with_commas(selection_len), char_count_display)
        } else {
            format!("{} characters", char_count_display)
        };
        let show_status_bar = self.show_status_bar;
        let encoding = self.encoding.to_string();
        let line_ending = self.line_ending.to_string();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(colors.background)
            .on_action(cx.listener(Self::export_pdf))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::paste))
            .child(
                // Main editor area
                div()
                    .flex_grow()
                    .p_2()
                // .text_color(gpui::black())  // Set text color to black
                .child(
                    Input::new(&self.input_state)
                        // No borders
                        .bordered(false)
                            .text_color(colors.accent_foreground)
                            .border_color(colors.border)
                            .h_full()
                    )
            )
            .children(if show_status_bar {
                Some(
                    // Status bar
                    div()
                        .h(px(24.0))
                        .bg(colors.muted)
                        .border_t_1()
                        .border_color(colors.border)
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .px_2()
                        .text_color(colors.muted_foreground)
                        .child(format!("Ln {}, Col {}", line, column))
                        .child(Self::separator(colors.border))
                        .child(count_display)
                        .child(Self::separator(colors.border))
                        .child(line_ending)
                        .child(Self::separator(colors.border))
                        .child(encoding)
                        .child(Self::separator(colors.border))
                        .child(format!("{} FPS", fps)),
                )
            } else {
                None
            })
    }
}

impl TextEditor {
    fn separator(color: Hsla) -> impl IntoElement {
        div()
            .h(px(14.0))
            .w(px(1.0))
            .mx(px(4.0))
            .bg(color)
    }

    fn format_with_commas(value: usize) -> String {
        let s = value.to_string();
        let mut out = String::new();
        for (i, ch) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        out.chars().rev().collect()
    }
}

/// Normalize tabs to two spaces.
fn normalize_tabs(content: &str) -> String {
    content.replace('\t', "  ")
}

#[cfg(test)]
mod tests {
    use super::normalize_tabs;

    #[test]
    fn test_normalize_tabs() {
        assert_eq!(normalize_tabs("hello\tworld"), "hello  world");
        assert_eq!(normalize_tabs("\t\t"), "    ");
        assert_eq!(normalize_tabs("no tabs"), "no tabs");
    }
}
