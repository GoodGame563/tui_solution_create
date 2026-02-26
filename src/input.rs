use crossterm::event::{KeyCode, KeyModifiers};
use crate::app::{App, AppState, SettingsCategory};

pub fn handle_input(app: &mut App, key: KeyCode, modifiers: KeyModifiers) -> Option<()> {
    if app.language_popup_open {
        handle_language_popup_input(app, key);
        return None;
    }

    if app.template_popup_open {
        handle_template_popup_input(app, key);
        return None;
    }

    match app.state {
        AppState::Input => {
            handle_input_state(app, key, modifiers);
        }
        AppState::Creating { .. } => {
            if key == KeyCode::Char('q') {
                return Some(());
            }
        }
        AppState::Done => {
            if key == KeyCode::Enter {
                app.reset_creation();
            } else if key == KeyCode::Char('q') {
                return Some(());
            }
        }
    }
    None
}

fn handle_language_popup_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.close_language_popup();
        }
        KeyCode::Enter => {
            app.confirm_language_selection();
        }
        KeyCode::Up => {
            app.move_language_selection(-1);
        }
        KeyCode::Down => {
            app.move_language_selection(1);
        }
        KeyCode::Backspace => {
            app.delete_language_search_char();
        }
        KeyCode::Char(c) => {
            app.insert_language_search_char(c);
        }
        _ => {}
    }
}

fn handle_template_popup_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.close_template_popup();
        }
        KeyCode::Enter => {
            app.save_template_popup();
        }
        KeyCode::Char(' ') => {
            app.toggle_template_popup_selection();
        }
        KeyCode::Up => {
            app.move_template_popup_selection(-1);
        }
        KeyCode::Down => {
            app.move_template_popup_selection(1);
        }
        _ => {}
    }
}

fn handle_input_state(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    match key {
        KeyCode::Tab => {
            if app.settings_edit_index.is_some() || app.list_edit_index.is_some() {
            } else if app.active_main_tab == 0 {
                app.next_main_tab();
            } else {
                app.previous_main_tab();
            }
        }
        KeyCode::BackTab => {
            if app.settings_edit_index.is_some() || app.list_edit_index.is_some() {
            } else if app.active_main_tab == 1 {
                app.previous_main_tab();
            }
        }
        KeyCode::Char('q') => {
            if app.settings_edit_index.is_none() && app.list_edit_index.is_none() && !app.input_active {
                std::process::exit(0);
            }
            if app.input_active && app.active_main_tab == 0 {
                app.insert_char('q');
            } else if app.active_main_tab == 1 {
                if app.settings_edit_index.is_some() {
                    app.insert_setting_char('q');
                } else if app.list_edit_index.is_some() {
                    app.insert_list_char('q');
                }
            }
        }
        KeyCode::Char('c') => {
            if app.settings_edit_index.is_some() || app.list_edit_index.is_some() || app.input_active {
                if app.input_active && app.active_main_tab == 0 {
                    app.insert_char('c');
                } else if app.active_main_tab == 1 {
                    if app.settings_edit_index.is_some() {
                        app.insert_setting_char('c');
                    } else if app.list_edit_index.is_some() {
                        app.insert_list_char('c');
                    }
                }
            } else if app.active_main_tab == 1 {
                let category = app.settings_categories[app.active_settings_category];
                if category == SettingsCategory::Git || category == SettingsCategory::Actions {
                    app.open_template_popup();
                }
            }
        }
        KeyCode::Enter => {
            if app.active_main_tab == 0 {
                if !app.url_input.is_empty() {
                    app.state = AppState::Creating { progress: 0.0, stage: 0 };
                    app.input_active = false;
                } else {
                    app.input_active = !app.input_active;
                }
            } else if app.active_main_tab == 1 {
                if app.settings_edit_index.is_some() {
                    app.stop_editing_setting();
                } else if app.list_edit_index.is_some() {
                    app.save_list_input();
                } else {
                    let category = app.settings_categories[app.active_settings_category];
                    if category == SettingsCategory::General {
                        app.start_editing_setting();
                    } else {
                        app.toggle_setting();
                    }
                }
            }
        }
        KeyCode::Esc => {
            if app.template_popup_open {
                app.close_template_popup();
            } else if app.language_popup_open {
                app.close_language_popup();
            } else if app.input_active {
                app.input_active = false;
            } else if app.active_main_tab == 1 {
                if app.settings_edit_index.is_some() {
                    app.stop_editing_setting();
                } else if app.list_edit_index.is_some() {
                    app.cancel_list_input();
                }
            }
        }
        KeyCode::Backspace => {
            if app.input_active && app.active_main_tab == 0 {
                app.delete_char();
            } else if app.active_main_tab == 1 {
                if app.settings_edit_index.is_some() {
                    app.delete_setting_char();
                } else if app.list_edit_index.is_some() {
                    app.delete_list_char();
                }
            }
        }
        KeyCode::Left => {
            if app.input_active && app.active_main_tab == 0 {
                app.move_cursor_left();
            } else if app.active_main_tab == 1 {
                if app.settings_edit_index.is_some() {
                    app.move_setting_cursor_left();
                } else if app.list_edit_index.is_some() {
                    app.move_list_cursor_left();
                } else {
                    app.previous_settings_category();
                }
            }
        }
        KeyCode::Right => {
            if app.input_active && app.active_main_tab == 0 {
                app.move_cursor_right();
            } else if app.active_main_tab == 1 {
                if app.settings_edit_index.is_some() {
                    app.move_setting_cursor_right();
                } else if app.list_edit_index.is_some() {
                    app.move_list_cursor_right();
                } else {
                    app.next_settings_category();
                }
            }
        }
        KeyCode::Up => {
            if app.template_popup_open {
                app.move_template_popup_selection(-1);
            } else if app.language_popup_open && !app.language_popup_in_modal {
                app.move_language_selection(-1);
            } else if app.input_active && app.active_main_tab == 0 {
            } else if app.active_main_tab == 1 {
                if app.settings_edit_index.is_some() {
                    app.stop_editing_setting();
                    app.move_settings_cursor(-1);
                } else if app.list_edit_index.is_none() {
                    app.move_settings_cursor(-1);
                }
            } else if app.active_main_tab == 0 {
                app.scroll_projects(-1);
            }
        }
        KeyCode::Down => {
            if app.template_popup_open {
                app.move_template_popup_selection(1);
            } else if app.language_popup_open && !app.language_popup_in_modal {
                app.move_language_selection(1);
            } else if app.input_active && app.active_main_tab == 0 {
            } else if app.active_main_tab == 1 {
                if app.settings_edit_index.is_some() {
                    app.stop_editing_setting();
                    app.move_settings_cursor(1);
                } else if app.list_edit_index.is_none() {
                    app.move_settings_cursor(1);
                }
            } else if app.active_main_tab == 0 {
                app.scroll_projects(1);
            }
        }
        KeyCode::Char('ё') | KeyCode::Char('`') => {
            if app.active_main_tab == 0 && !app.input_active {
                app.open_language_popup();
            }
        }
        KeyCode::Char('v') if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.input_active && app.active_main_tab == 0 {
                app.paste_from_clipboard();
            } else if app.active_main_tab == 1 {
                if app.settings_edit_index.is_some() {
                    app.paste_setting_from_clipboard();
                } else if app.list_edit_index.is_some() {
                    app.paste_list_from_clipboard();
                }
            }
        }
        KeyCode::Char(' ') => {
            if app.template_popup_open {
                app.toggle_template_popup_selection();
            } else if app.active_main_tab == 1 && app.settings_edit_index.is_none() && app.list_edit_index.is_none() {
                app.toggle_setting();
            }
        }
        KeyCode::Char(c) => {
            if app.template_popup_open {
            } else if app.language_popup_open && !app.language_popup_in_modal {
                app.insert_language_search_char(c);
            } else if app.input_active && app.active_main_tab == 0 {
                app.insert_char(c);
            } else if app.active_main_tab == 1 {
                if app.settings_edit_index.is_some() {
                    app.insert_setting_char(c);
                } else if app.list_edit_index.is_some() {
                    app.insert_list_char(c);
                }
            }
        }
        _ => {}
    }
}
