//! Menu building for the workspace.

use gpui::*;
use gpui_component::Theme;
use gpui_component::ThemeRegistry;
use gpui_component::menu::{DropdownMenu, PopupMenuItem};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Copy, Cut, SelectAll};

use crate::{DecreaseFontSizeAction, ExitAppAction, ExportPdfAction, FindAction, GoToLineAction, IncreaseFontSizeAction, InsertDateTimeAction, NewFileAction, OpenFileDialogAction, ReplaceAction, ResetFontSizeAction, SaveFileAction, SaveFileAsAction};
use crate::editor::{UndoAction, RedoAction, NormalizePasteAction};
use super::Workspace;

/// Shorthand for accessing workspace from menu handlers.
macro_rules! with_workspace {
    ($window:expr, $app:expr, |$this:ident, $win:ident, $cx:ident| $body:expr) => {{
        gpui_component::Root::update($window, $app, |root, $win, cx_root| {
            if let Ok(workspace) = root.view().clone().downcast::<Workspace>() {
                let _ = workspace.update(cx_root, |$this, $cx| $body);
            }
        });
    }};
}

impl Workspace {
    pub(super) fn build_file_menu(&self) -> impl IntoElement {
        let recent_files = self.settings.recent_files.clone();
        let restore_last_file = self.settings.restore_last_file;
        Button::new("menu:file")
            .label("File")
            .text()
            .dropdown_caret(true)
            .dropdown_menu(move |menu, window, cx_menu| {
                let recent_files = recent_files.clone();
                menu
                    .item(PopupMenuItem::new("New").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.new_file(window, cx);
                        });
                    }).action(Box::new(NewFileAction)))
                    .item(PopupMenuItem::new("Open...").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.open_dialog(window, cx);
                        });
                    }).action(Box::new(OpenFileDialogAction)))
                    .submenu("Open Recent", window, cx_menu, {
                        let recent_files = recent_files.clone();
                        move |submenu, _window, _cx_submenu| {
                            let submenu = recent_files.iter().fold(submenu, |submenu, path| {
                                let display = path.to_string_lossy().to_string();
                                let path = path.clone();
                                submenu.item(PopupMenuItem::new(display).on_click(move |_, window, app| {
                                    let path = path.clone();
                                    with_workspace!(window, app, |this, window, cx| {
                                        this.open_file_checked(path, window, cx);
                                    });
                                }))
                            });
                            let submenu = if recent_files.is_empty() {
                                submenu.item(PopupMenuItem::new("(empty)"))
                            } else {
                                submenu.item(PopupMenuItem::separator())
                            };
                            submenu.item(PopupMenuItem::new("Clear Recent Files").on_click(|_, window, app| {
                                with_workspace!(window, app, |this, _window, cx| {
                                    this.clear_recent_files(cx);
                                });
                            }))
                        }
                    })
                    .item(PopupMenuItem::new("Reopen Last File on Startup").checked(restore_last_file).on_click(|_, window, app| {
                        with_workspace!(window, app, |this, _window, cx| {
                            this.toggle_restore_last_file(cx);
                        });
                    }))
                    .item(PopupMenuItem::separator())
                    .item(PopupMenuItem::new("Save").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.save_file(window, cx);
                        });
                    }).action(Box::new(SaveFileAction)))
                    .item(PopupMenuItem::new("Save As...").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.save_as_dialog(window, cx);
                        });
                    }).action(Box::new(SaveFileAsAction)))
                    .item(PopupMenuItem::separator())
                    .item(PopupMenuItem::new("Export to PDF...").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.export_pdf(&ExportPdfAction, window, cx));
                        });
                    }).action(Box::new(ExportPdfAction)))
                    .item(PopupMenuItem::separator())
                    .item(PopupMenuItem::new("Exit").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.exit_app(window, cx);
                        });
                    }).action(Box::new(ExitAppAction)))
            })
    }

    pub(super) fn build_edit_menu(&self) -> impl IntoElement {
        Button::new("menu:edit")
            .label("Edit")
            .text()
            .dropdown_caret(true)
            .dropdown_menu(|menu, _window, _cx_menu| {
                menu
                    .item(PopupMenuItem::new("Undo").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.undo(&UndoAction, window, cx));
                        });
                    }).action(Box::new(UndoAction)))
                    .item(PopupMenuItem::new("Redo").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.redo(&RedoAction, window, cx));
                        });
                    }).action(Box::new(RedoAction)))
                    .item(PopupMenuItem::separator())
                    .item(PopupMenuItem::new("Cut").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.cut(window, cx));
                        });
                    }).action(Box::new(Cut)))
                    .item(PopupMenuItem::new("Copy").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.copy(window, cx));
                        });
                    }).action(Box::new(Copy)))
                    .item(PopupMenuItem::new("Paste").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.paste(&NormalizePasteAction, window, cx));
                        });
                    }).action(Box::new(NormalizePasteAction)))
                    .item(PopupMenuItem::separator())
                    .item(PopupMenuItem::new("Find").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.open_search(window, cx));
                        });
                    }).action(Box::new(FindAction)))
                    .item(PopupMenuItem::new("Replace").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.open_replace(window, cx));
                        });
                    }).action(Box::new(ReplaceAction)))
                    .item(PopupMenuItem::new("Go to Line...").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.open_goto_line(window, cx));
                        });
                    }).action(Box::new(GoToLineAction)))
                    .item(PopupMenuItem::separator())
                    .item(PopupMenuItem::new("Insert Date/Time").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.insert_datetime(window, cx));
                        });
                    }).action(Box::new(InsertDateTimeAction)))
                    .item(PopupMenuItem::new("Select All").on_click(|_, window, app| {
                        with_workspace!(window, app, |this, window, cx| {
                            this.with_editor(cx, |ed, cx| ed.select_all(window, cx));
                        });
                    }).action(Box::new(SelectAll)))
            })
    }

    pub(super) fn build_view_menu(&self, soft_wrap_enabled: bool, show_status_bar: bool, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("menu:view")
            .label("View")
            .text()
            .dropdown_caret(true)
            .dropdown_menu({
                move |menu, window, cx_menu| {
                    menu
                        .item(PopupMenuItem::new("Word Wrap").checked(soft_wrap_enabled).on_click(|_, window, app| {
                            with_workspace!(window, app, |this, window, cx| {
                                this.with_editor(cx, |ed, cx| ed.toggle_soft_wrap(window, cx));
                            });
                        }))
                        .item(PopupMenuItem::new("Status Bar").checked(show_status_bar).on_click(|_, window, app| {
                            with_workspace!(window, app, |this, window, cx| {
                                this.with_editor(cx, |ed, cx| ed.toggle_status_bar(window, cx));
                            });
                        }))
                        .item(PopupMenuItem::separator())
                        .item(PopupMenuItem::new("Increase Font Size").on_click(|_, window, app| {
                            with_workspace!(window, app, |this, _window, cx| {
                                this.increase_font_size(cx);
                            });
                        }).action(Box::new(IncreaseFontSizeAction)))
                        .item(PopupMenuItem::new("Decrease Font Size").on_click(|_, window, app| {
                            with_workspace!(window, app, |this, _window, cx| {
                                this.decrease_font_size(cx);
                            });
                        }).action(Box::new(DecreaseFontSizeAction)))
                        .item(PopupMenuItem::new("Reset Font Size").on_click(|_, window, app| {
                            with_workspace!(window, app, |this, _window, cx| {
                                this.reset_font_size(cx);
                            });
                        }).action(Box::new(ResetFontSizeAction)))
                        .item(PopupMenuItem::separator())
                        .submenu("Theme", window, cx_menu, |submenu, _window, cx_submenu| {
                            let mut theme_names: Vec<String> = ThemeRegistry::global(cx_submenu)
                                .themes()
                                .keys()
                                .map(|s| s.to_string())
                                .collect();
                            theme_names.sort();
                            let active_theme = Theme::global(cx_submenu).theme_name().clone();

                            theme_names.into_iter().fold(
                                submenu.max_h(px(320.0)).scrollable(true),
                                move |submenu, name| {
                                    let is_active = active_theme == name;
                                    submenu.item(
                                        PopupMenuItem::new(name.clone())
                                            .checked(is_active)
                                            .on_click({
                                                let theme_name = name.clone();
                                                move |_, window, app| {
                                                    let name = theme_name.clone();
                                                    with_workspace!(window, app, |this, _window, cx| {
                                                        this.apply_theme(name, cx);
                                                    });
                                                }
                                            }),
                                    )
                                },
                            )
                        })
                        .item(PopupMenuItem::separator())
                        .item(PopupMenuItem::new("License").on_click(|_, window, app| {
                            with_workspace!(window, app, |this, window, cx| {
                                this.open_license(window, cx);
                            });
                        }))
                }
            })
    }

    pub(super) fn build_menu_bar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global_mut(cx);
        let palette = theme.colors;
        
        let (soft_wrap_enabled, show_status_bar) = if let Some(editor) = &self.editor_entity {
            let ed = editor.read(cx);
            (ed.soft_wrap, ed.show_status_bar)
        } else {
            (true, true)
        };

        let file_menu = self.build_file_menu();
        let edit_menu = self.build_edit_menu();
        let view_menu = self.build_view_menu(soft_wrap_enabled, show_status_bar, window, cx);

        div()
            .flex()
            .relative()
            .w_full()
            .h(px(32.0))
            .border_b_1()
            .border_color(palette.border)
            .bg(palette.muted)
            .px_2()
            .items_center()
            .gap(px(8.0))
            .child(file_menu)
            .child(edit_menu)
            .child(view_menu)
    }
}
