//! File operations for the workspace (open, save, save-as dialogs).

use gpui::*;
use gpui_component::Root;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};
use rfd::{AsyncFileDialog, AsyncMessageDialog, MessageButtons, MessageDialogResult};

use super::Workspace;

/// Access workspace from async context. Returns None if downcast fails.
fn with_workspace_async<R>(
    cx: &mut AsyncWindowContext,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R,
) -> Option<R> {
    cx.update(|window, app| {
        Root::update(window, app, |root, window, cx_root| {
            root.view().clone().downcast::<Workspace>().ok().map(|workspace| {
                workspace.update(cx_root, |this, cx_ws| f(this, window, cx_ws))
            })
        })
    })
    .ok()
    .flatten()
}

impl Workspace {
    /// Open file picker (checks for unsaved changes first).
    pub fn open_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.handle_unsaved_changes(window, cx, |this, window, cx| {
            this.open_dialog_internal(window, cx);
        });
    }

    pub fn open_dialog_internal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, move |_this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                debug!("Opening file dialog");
                let dialog_task = cx.background_spawn(async move {
                    if let Some(file) = AsyncFileDialog::new().pick_file().await {
                        let path = file.path().to_path_buf();
                        match fs::read_to_string(&path) {
                            Ok(contents) => Some((path, contents)),
                            Err(err) => {
                                warn!(path = ?path, error = %err, "Failed to read file");
                                None
                            }
                        }
                    } else {
                        None
                    }
                });

                if let Some((path, contents)) = dialog_task.await {
                    debug!(path = ?path, bytes = contents.len(), "File selected from dialog");
                    with_workspace_async(&mut cx, |this, window, cx_ws| {
                        debug!(has_editor = this.editor_entity.is_some(), "Updating workspace with file");
                        this.current_file = Some(path.clone());
                        
                        // Make sure to reset editor state completely
                        if let Some(editor) = &this.editor_entity {
                            let contents = contents.clone();
                            editor.update(cx_ws, |ed, cx_ed| {
                                let _ = ed.open_file(path.clone(), window, cx_ed, Some(contents));
                            });
                        } else {
                            warn!("Editor entity missing when opening file");
                        }
                        this.update_title(window, cx_ws);
                    });
                } else {
                    debug!("Open dialog canceled");
                    let _ = cx.update(|_, _| {});
                }
            }
        })
        .detach();
    }

    /// Save file, or show Save As if untitled.
    pub fn save_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(task) = self.save_file_task(window, cx) {
            task.detach();
        }
    }

    pub fn save_file_task(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<Task<bool>> {
        if self.current_file.is_none() {
            return Some(self.save_as_dialog_task(window, cx));
        }

        let path = self.current_file.clone()?;

        Some(cx.spawn_in(window, move |_this: WeakEntity<Self>, cx_async: &mut AsyncWindowContext| {
            let mut cx = cx_async.clone();
            async move {
                let contents = Self::get_editor_text_async(&mut cx);
                Self::write_file_and_update(&mut cx, path, contents).await
            }
        }))
    }

    /// Show Save As dialog.
    pub fn save_as_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_as_dialog_task(window, cx).detach();
    }

    pub fn save_as_dialog_task(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Task<bool> {
        cx.spawn_in(window, move |_this: WeakEntity<Self>, cx_async: &mut AsyncWindowContext| {
            let mut cx = cx_async.clone();
            async move {
                debug!("Opening save-as dialog");
                let dialog_task = cx.background_spawn(async move {
                    AsyncFileDialog::new()
                        .save_file()
                        .await
                        .map(|file| file.path().to_path_buf())
                });

                if let Some(path) = dialog_task.await {
                    debug!(path = ?path, "Save-as path selected");
                    
                    // Update editor's file path first
                    with_workspace_async(&mut cx, |this, _window, cx_ws| {
                        if let Some(editor) = &this.editor_entity {
                            editor.update(cx_ws, |ed, _| {
                                ed.current_file = Some(path.clone());
                            });
                        }
                    });
                    
                    let contents = Self::get_editor_text_async(&mut cx);
                    Self::write_file_and_update(&mut cx, path, contents).await
                } else {
                    debug!("Save-as dialog canceled");
                    let _ = cx.update(|_, _| {});
                    false
                }
            }
        })
    }

    fn get_editor_text_async(cx: &mut AsyncWindowContext) -> String {
        with_workspace_async(cx, |this, _window, cx_ws| {
            this.get_editor_text(cx_ws)
        })
        .unwrap_or_default()
    }

    async fn write_file_and_update(cx: &mut AsyncWindowContext, path: PathBuf, contents: String) -> bool {
        let path_for_write = path.clone();
        let success = cx.background_spawn(async move {
            match fs::write(&path_for_write, contents) {
                Ok(_) => {
                    info!(path = ?path_for_write, "File saved");
                    true
                }
                Err(err) => {
                    warn!(path = ?path_for_write, error = %err, "Failed to save file");
                    false
                }
            }
        }).await;

        if success {
            with_workspace_async(cx, |this, window, cx_ws| {
                this.current_file = Some(path.clone());
                
                // Mark editor clean
                if let Some(editor) = &this.editor_entity {
                    editor.update(cx_ws, |ed, _| ed.mark_clean());
                }
                
                this.update_title(window, cx_ws);
                cx_ws.notify();
            });
            true
        } else {
            let _ = cx.update(|_, _| {});
            false
        }
    }

    pub(super) fn get_editor_text(&self, cx: &mut Context<Self>) -> String {
        if let Some(editor) = &self.editor_entity {
            editor.update(cx, |ed, cx_ed| {
                ed.input_state.read(cx_ed).value().to_string()
            })
        } else {
            String::new()
        }
    }

    /// Prompt for unsaved changes, then run continuation.
    pub fn handle_unsaved_changes<F>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        continuation: F,
    ) where
        F: FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) + 'static + Send,
    {
        // Check setting
        if !self.settings.enable_unsaved_changes_protection {
            continuation(self, window, cx);
            return;
        }

        // Check dirty state
        let is_dirty = if let Some(editor) = &self.editor_entity {
            editor.read(cx).is_dirty
        } else {
            false
        };

        if !is_dirty {
            continuation(self, window, cx);
            return;
        }

        // Show dialog
        cx.spawn_in(window, move |_this, cx_async: &mut AsyncWindowContext| {
            let mut cx = cx_async.clone();
            async move {
                let result = AsyncMessageDialog::new()
                    .set_title("Unsaved Changes")
                    .set_description("You have unsaved changes. Do you want to save them?")
                    .set_buttons(MessageButtons::YesNoCancel)
                    .show()
                    .await;

                match result {
                    MessageDialogResult::Yes => {
                        // User wants to save
                        let task_opt = with_workspace_async(&mut cx, |this, window, cx_ws| {
                            this.save_file_task(window, cx_ws)
                        }).flatten();
                        
                        // Wait for save logic
                        if let Some(save_task) = task_opt {
                            if save_task.await {
                                // Save successful, proceed
                                with_workspace_async(&mut cx, |this, window, cx_ws| {
                                    continuation(this, window, cx_ws);
                                });
                            }
                        }
                    }
                    MessageDialogResult::No => {
                        // Discard changes
                        with_workspace_async(&mut cx, |this, window, cx_ws| {
                            continuation(this, window, cx_ws);
                        });
                    }
                    _ => {} // Cancel, do nothing
                }
            }
        }).detach();
    }
}
