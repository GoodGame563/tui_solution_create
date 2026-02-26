use crate::app::{App, SettingsCategory};
use crate::structs::ToggleMode;

pub fn get_settings_for_category(
    app: &App,
) -> Vec<(
    &'static str,
    &'static str,
    bool,
    bool,
    bool,
    Option<String>,
    Vec<String>,
    bool,
    String,
)> {
    let category = app.settings_categories[app.active_settings_category];
    match category {
        SettingsCategory::General => {
            vec![
                (
                    "Recipes URL",
                    "Path to recipes directory",
                    !app.settings.recipes_url.is_empty(),
                    false,
                    app.settings_edit_index.map(|(_, c)| c) == Some(0),
                    Some(app.settings.recipes_url.clone()),
                    vec![],
                    false,
                    String::new(),
                ),
                (
                    "Solutions URL",
                    "Path to solutions directory",
                    !app.settings.solutions_url.is_empty(),
                    false,
                    app.settings_edit_index.map(|(_, c)| c) == Some(1),
                    Some(app.settings.solutions_url.clone()),
                    vec![],
                    false,
                    String::new(),
                ),
            ]
        }
        SettingsCategory::Git => {
            vec![
                (
                    "Init Git",
                    "Initialize git repository",
                    app.is_toggle_enabled(SettingsCategory::Git, 0),
                    app.settings.init_git.mode == ToggleMode::YesSome,
                    false,
                    None,
                    app.get_toggle_recipes(SettingsCategory::Git, 0),
                    app.is_list_editing(0),
                    app.get_list_input().to_string(),
                ),
                (
                    "Create .gitignore",
                    "Create local .gitignore file",
                    app.is_toggle_enabled(SettingsCategory::Git, 1),
                    app.settings.create_local_gitignore.mode == ToggleMode::YesSome,
                    false,
                    None,
                    app.get_toggle_recipes(SettingsCategory::Git, 1),
                    app.is_list_editing(1),
                    app.get_list_input().to_string(),
                ),
            ]
        }
        SettingsCategory::Actions => {
            vec![
                (
                    "Open Terminal",
                    "Open terminal after creation",
                    app.is_toggle_enabled(SettingsCategory::Actions, 0),
                    app.settings.open_terminal.mode == ToggleMode::YesSome,
                    false,
                    None,
                    app.get_toggle_recipes(SettingsCategory::Actions, 0),
                    app.is_list_editing(0),
                    app.get_list_input().to_string(),
                ),
                (
                    "Open IDE",
                    "Open IDE after creation",
                    app.is_toggle_enabled(SettingsCategory::Actions, 1),
                    app.settings.open_ide.mode == ToggleMode::YesSome,
                    false,
                    None,
                    app.get_toggle_recipes(SettingsCategory::Actions, 1),
                    app.is_list_editing(1),
                    app.get_list_input().to_string(),
                ),
            ]
        }
    }
}
