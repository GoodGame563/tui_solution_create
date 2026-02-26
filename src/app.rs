use crate::structs::{AppConfig, ToggleMode, ToggleWithList};
use crate::utils::get_all_config;
use ratatui::prelude::Color;
use std::{fs, time::Duration};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsCategory {
    General,
    Git,
    Actions,
}

impl SettingsCategory {
    pub fn all() -> Vec<SettingsCategory> {
        vec![
            SettingsCategory::General,
            SettingsCategory::Git,
            SettingsCategory::Actions,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            SettingsCategory::General => "General",
            SettingsCategory::Git => "Git",
            SettingsCategory::Actions => "Actions",
        }
    }
}

pub enum AppState {
    Input,
    Creating { progress: f64, stage: usize },
    Done,
}

pub struct App {
    pub main_tabs: Vec<&'static str>,
    pub active_main_tab: usize,
    pub url_input: String,
    pub cursor_position: usize,
    pub input_active: bool,
    pub languages: Vec<String>,
    pub selected_language_index: usize,
    pub projects: Vec<(String, String)>,
    pub projects_scroll: usize,
    pub config: AppConfig,
    pub settings_categories: Vec<SettingsCategory>,
    pub active_settings_category: usize,
    pub settings_cursor: usize,
    pub settings_edit_index: Option<(usize, usize)>,
    pub settings_edit_cursor: usize,
    pub list_edit_index: Option<usize>,
    pub list_input_cursor: usize,
    pub state: AppState,
    pub creation_stages: Vec<&'static str>,
    pub language_popup_open: bool,
    pub language_popup_in_modal: bool,
    pub language_search: String,
    pub language_search_cursor: usize,
    pub language_filtered_index: usize,
    pub status_message: Option<(String, Duration)>,
    pub template_popup_open: bool,
    pub template_popup_recipes: Vec<String>,
    pub template_popup_selected: Vec<bool>,
    pub template_popup_scroll: usize,
    pub template_popup_source: Option<(SettingsCategory, usize)>,
}

impl App {
    pub fn new() -> Self {
        let config: AppConfig = match confy::load("tui-solution-create", None) {
            Ok(c) => c,
            Err(e) => {
                let cfg = AppConfig::default();
                _ = confy::store("tui-solution-create", None, &cfg);
                cfg
            }
        };
        let languages = Self::load_languages(&config);
        Self {
            main_tabs: vec!["Recipes", "Settings"],
            active_main_tab: 0,
            url_input: String::new(),
            cursor_position: 0,
            input_active: false,
            languages: languages.clone(),
            selected_language_index: 0,
            projects: vec![
                ("Rust".to_string(), "tui-app".to_string()),
                ("Python".to_string(), "data-scraper".to_string()),
                ("Go".to_string(), "web-server".to_string()),
                ("Rust".to_string(), "cli-tool".to_string()),
                ("Python".to_string(), "automation-bot".to_string()),
                ("Go".to_string(), "api-gateway".to_string()),
                ("Rust".to_string(), "blockchain-node".to_string()),
            ],
            projects_scroll: 0,
            config: config,
            settings_categories: SettingsCategory::all(),
            active_settings_category: 0,
            settings_cursor: 0,
            settings_edit_index: None,
            settings_edit_cursor: 0,
            list_edit_index: None,
            list_input_cursor: 0,
            state: AppState::Input,
            creation_stages: vec![
                "Initializing project...",
                "Cloning repository...",
                "Installing dependencies...",
                "Configuring environment...",
                "Building solution...",
                "Finalizing...",
            ],
            language_popup_open: false,
            language_popup_in_modal: false,
            language_search: String::new(),
            language_search_cursor: 0,
            language_filtered_index: 0,
            status_message: None,
            template_popup_open: false,
            template_popup_recipes: Vec::new(),
            template_popup_selected: Vec::new(),
            template_popup_scroll: 0,
            template_popup_source: None,
        }
    }

    fn load_languages(config: &AppConfig) -> Vec<String> {
        let mut langs = Vec::new();
        for config in get_all_config(&config.recipes_url){
            langs.push(config.name);
        }
        if langs.is_empty() {
            langs.push("None".to_string());
        }
        langs.sort();
        langs
    }

    pub fn selected_language(&self) -> String {
        self.languages
            .get(self.selected_language_index)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string())
    }


    pub fn open_language_popup(&mut self) {
        self.language_popup_open = true;
        self.language_popup_in_modal = false;
        self.language_search.clear();
        self.languages = Self::load_languages(&self.config);
        self.language_search_cursor = 0;
        self.language_filtered_index = 0;
    }

    pub fn close_language_popup(&mut self) {
        self.language_popup_open = false;
        self.language_popup_in_modal = false;
        self.language_search.clear();
        self.language_search_cursor = 0;
        self.language_filtered_index = 0;
    }

    pub fn confirm_language_selection(&mut self) {
        let filtered = self.get_filtered_languages();
        if let Some((orig_idx, _)) = filtered.get(self.language_filtered_index) {
            self.selected_language_index = *orig_idx;
        }
        self.language_popup_open = false;
        self.language_popup_in_modal = false;
        self.language_search.clear();
        self.language_search_cursor = 0;
        self.language_filtered_index = 0;
    }

    pub fn open_recipes_popup(&mut self) {
        let category = self.settings_categories[self.active_settings_category];
        let idx = self.settings_cursor;

        let mut recipes = Vec::new();
        for config in get_all_config(&self.config.recipes_url){
            recipes.push(config.name);
        }
        recipes.sort();

        let current_selected = if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
            toggle.list.clone()
        } else {
            Vec::new()
        };

        self.template_popup_recipes = recipes;
        self.template_popup_selected = self
            .template_popup_recipes
            .iter()
            .map(|r| current_selected.contains(r))
            .collect();
        self.template_popup_scroll = 0;
        self.template_popup_source = Some((category, idx));
        self.template_popup_open = true;
    }

    pub fn close_recipes_popup(&mut self) {
        self.template_popup_open = false;
        self.template_popup_recipes.clear();
        self.template_popup_selected.clear();
        self.template_popup_source = None;
    }

    pub fn save_recipes_popup(&mut self) {
        let selected: Vec<String> = self
            .template_popup_recipes
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                self.template_popup_selected
                    .get(*i)
                    .copied()
                    .unwrap_or(false)
            })
            .map(|(_, r)| r.clone())
            .collect();
        let any_selected = !selected.is_empty();

        if let Some((category, idx)) = self.template_popup_source {
            if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                toggle.list = selected;
                toggle.mode = if any_selected {
                    ToggleMode::YesSome
                } else {
                    ToggleMode::No
                };
            }
        }

        self.close_recipes_popup()
    }

    pub fn toggle_recipes_popup_selection(&mut self) {
        if !self.template_popup_selected.is_empty() {
            let idx = self.template_popup_scroll;
            if idx < self.template_popup_selected.len() {
                self.template_popup_selected[idx] = !self.template_popup_selected[idx];
            }
        }
    }

    pub fn move_recipes_popup_selection(&mut self, direction: i8) {
        if direction > 0 {
            if self.template_popup_scroll < self.template_popup_recipes.len().saturating_sub(1) {
                self.template_popup_scroll += 1;
            }
        } else {
            if self.template_popup_scroll > 0 {
                self.template_popup_scroll -= 1;
            }
        }
    }

    pub fn get_filtered_languages(&self) -> Vec<(usize, String)> {
        if self.language_search.is_empty() {
            self.languages
                .iter()
                .enumerate()
                .map(|(i, lang)| (i, lang.clone()))
                .collect()
        } else {
            self.languages
                .iter()
                .enumerate()
                .filter(|(_, lang)| {
                    lang.to_lowercase()
                        .contains(&self.language_search.to_lowercase())
                })
                .map(|(i, lang)| (i, lang.clone()))
                .collect()
        }
    }

    pub fn move_language_selection(&mut self, direction: i8) {
        let filtered = self.get_filtered_languages();
        if filtered.is_empty() {
            return;
        }
        if direction > 0 {
            self.language_filtered_index = (self.language_filtered_index + 1) % filtered.len();
        } else {
            self.language_filtered_index = if self.language_filtered_index == 0 {
                filtered.len() - 1
            } else {
                self.language_filtered_index - 1
            };
        }
    }

    pub fn insert_language_search_char(&mut self, c: char) {
        let mut chars: Vec<char> = self.language_search.chars().collect();
        chars.insert(self.language_search_cursor, c);
        self.language_search = chars.into_iter().collect();
        self.language_search_cursor += 1;
    }

    pub fn delete_language_search_char(&mut self) {
        if self.language_search_cursor > 0 {
            self.language_search_cursor -= 1;
            let mut chars: Vec<char> = self.language_search.chars().collect();
            if self.language_search_cursor < chars.len() {
                chars.remove(self.language_search_cursor);
                self.language_search = chars.into_iter().collect();
            }
        }
    }

    pub fn next_main_tab(&mut self) {
        self.active_main_tab = (self.active_main_tab + 1) % self.main_tabs.len();
    }

    pub fn previous_main_tab(&mut self) {
        if self.active_main_tab > 0 {
            self.active_main_tab -= 1;
        } else {
            self.active_main_tab = self.main_tabs.len() - 1;
        }
    }

    pub fn next_settings_category(&mut self) {
        self.active_settings_category =
            (self.active_settings_category + 1) % self.settings_categories.len();
        self.settings_cursor = 0;
        self.list_edit_index = None;
    }

    pub fn previous_settings_category(&mut self) {
        if self.active_settings_category > 0 {
            self.active_settings_category -= 1;
        } else {
            self.active_settings_category = self.settings_categories.len() - 1;
        }
        self.settings_cursor = 0;
        self.list_edit_index = None;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            let mut chars: Vec<char> = self.url_input.chars().collect();
            if self.cursor_position < chars.len() {
                chars.remove(self.cursor_position);
                self.url_input = chars.into_iter().collect();
            }
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let mut chars: Vec<char> = self.url_input.chars().collect();
        chars.insert(self.cursor_position, c);
        self.url_input = chars.into_iter().collect();
        self.cursor_position += 1;
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        let len = self.url_input.chars().count();
        if self.cursor_position < len {
            self.cursor_position += 1;
        }
    }

    pub fn paste_from_clipboard(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                for c in text.chars() {
                    let mut chars: Vec<char> = self.url_input.chars().collect();
                    chars.insert(self.cursor_position, c);
                    self.url_input = chars.into_iter().collect();
                    self.cursor_position += 1;
                }
            }
        }
    }

    pub fn paste_setting_from_clipboard(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                let mut cursor = self.settings_edit_cursor;
                if let Some(field) = self.get_editable_field_mut() {
                    for c in text.chars() {
                        let mut chars: Vec<char> = field.chars().collect();
                        chars.insert(cursor, c);
                        *field = chars.into_iter().collect();
                        cursor += 1;
                    }
                }
                self.settings_edit_cursor = cursor;
            }
        }
    }

    pub fn paste_list_from_clipboard(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                let category = self.settings_categories[self.active_settings_category];
                let idx_opt = self.list_edit_index;
                let mut cursor = self.list_input_cursor;
                if let Some(idx) = idx_opt {
                    if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                        for c in text.chars() {
                            let mut chars: Vec<char> = toggle.list_input.chars().collect();
                            chars.insert(cursor, c);
                            toggle.list_input = chars.into_iter().collect();
                            cursor += 1;
                        }
                    }
                }
                self.list_input_cursor = cursor;
            }
        }
    }

    pub fn scroll_projects(&mut self, direction: i8) {
        if direction > 0 {
            if self.projects_scroll < self.projects.len().saturating_sub(1) {
                self.projects_scroll += 1;
            }
        } else {
            if self.projects_scroll > 0 {
                self.projects_scroll -= 1;
            }
        }
    }

    pub fn get_settings_items_count(&self) -> usize {
        match self.settings_categories[self.active_settings_category] {
            SettingsCategory::General => 2,
            SettingsCategory::Git => 2,
            SettingsCategory::Actions => 2,
        }
    }

    pub fn move_settings_cursor(&mut self, direction: i8) {
        if self.list_edit_index.is_some() {
            return;
        }
        let count = self.get_settings_items_count();
        if direction > 0 {
            if self.settings_cursor < count - 1 {
                self.settings_cursor += 1;
            }
        } else {
            if self.settings_cursor > 0 {
                self.settings_cursor -= 1;
            }
        }
        self.deactivate_all_list_inputs();
    }

    fn deactivate_all_list_inputs(&mut self) {
        self.config.init_git.list_input_active = false;
        self.config.create_local_gitignore.list_input_active = false;
        self.config.open_terminal.list_input_active = false;
        self.config.open_ide.list_input_active = false;
    }

    pub fn get_toggle_for_setting(
        &mut self,
        category: SettingsCategory,
        index: usize,
    ) -> Option<&mut ToggleWithList> {
        match category {
            SettingsCategory::Git => match index {
                0 => Some(&mut self.config.init_git),
                1 => Some(&mut self.config.create_local_gitignore),
                _ => None,
            },
            SettingsCategory::Actions => match index {
                0 => Some(&mut self.config.open_terminal),
                1 => Some(&mut self.config.open_ide),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn toggle_setting(&mut self) {
        self.deactivate_all_list_inputs();
        let category = self.settings_categories[self.active_settings_category];
        let settings_cursor = self.settings_cursor;

        match category {
            SettingsCategory::General => match settings_cursor {
                0 => {
                    self.config.recipes_url = if self.config.recipes_url.is_empty() {
                        "recipes".to_string()
                    } else {
                        String::new()
                    };
                }
                1 => {
                    self.config.solutions_url = if self.config.solutions_url.is_empty() {
                        "solutions".to_string()
                    } else {
                        String::new()
                    };
                }
                _ => {}
            },
            SettingsCategory::Git | SettingsCategory::Actions => {
                if let Some(toggle) = self.get_toggle_for_setting(category, settings_cursor) {
                    toggle.mode = match toggle.mode {
                        ToggleMode::No => ToggleMode::Yes,
                        ToggleMode::Yes => ToggleMode::No,
                        ToggleMode::YesSome => ToggleMode::No,
                    };
                    toggle.list_input_active = false;
                }
                self.list_edit_index = None;
            }
        }
    }

    pub fn update_creation(&mut self) {
        if let AppState::Creating { progress, stage } = &mut self.state {
            *progress += 0.02;
            if *progress >= 1.0 {
                *progress = 0.0;
                *stage += 1;
                if *stage >= self.creation_stages.len() {
                    self.state = AppState::Done;
                    self.projects
                        .insert(0, (self.selected_language(), self.url_input.clone()));
                    self.set_status(
                        &format!("Project '{}' created!", self.url_input),
                        Color::Green,
                    );
                    self.url_input.clear();
                    self.cursor_position = 0;
                }
            }
        }
    }

    pub fn reset_creation(&mut self) {
        self.state = AppState::Input;
    }

    pub fn start_editing_setting(&mut self) {
        let category = self.settings_categories[self.active_settings_category];
        if category == SettingsCategory::General {
            match self.settings_cursor {
                0 => {
                    self.settings_edit_index = Some((0, self.settings_cursor));
                    self.settings_edit_cursor = self.config.recipes_url.len();
                }
                1 => {
                    self.settings_edit_index = Some((1, self.settings_cursor));
                    self.settings_edit_cursor = self.config.solutions_url.len();
                }
                _ => {}
            }
        }
    }

    pub fn stop_editing_setting(&mut self) {
        self.settings_edit_index = None;
    }

    fn get_editable_field(&self) -> Option<&String> {
        match self.settings_edit_index {
            Some((0, _)) => Some(&self.config.recipes_url),
            Some((1, _)) => Some(&self.config.solutions_url),
            _ => None,
        }
    }

    fn get_editable_field_mut(&mut self) -> Option<&mut String> {
        match self.settings_edit_index {
            Some((0, _)) => Some(&mut self.config.recipes_url),
            Some((1, _)) => Some(&mut self.config.solutions_url),
            _ => None,
        }
    }

    pub fn insert_setting_char(&mut self, c: char) {
        let cursor = self.settings_edit_cursor;
        if let Some(field) = self.get_editable_field_mut() {
            let mut chars: Vec<char> = field.chars().collect();
            chars.insert(cursor, c);
            *field = chars.into_iter().collect();
            self.settings_edit_cursor += 1;
        }
    }

    pub fn delete_setting_char(&mut self) {
        if self.settings_edit_cursor > 0 {
            let cursor = self.settings_edit_cursor - 1;
            if let Some(field) = self.get_editable_field_mut() {
                let mut chars: Vec<char> = field.chars().collect();
                if cursor < chars.len() {
                    chars.remove(cursor);
                    *field = chars.into_iter().collect();
                }
                self.settings_edit_cursor -= 1;
            }
        }
    }

    pub fn move_setting_cursor_left(&mut self) {
        if self.settings_edit_cursor > 0 {
            self.settings_edit_cursor -= 1;
        }
    }

    pub fn move_setting_cursor_right(&mut self) {
        if let Some(field) = self.get_editable_field() {
            let len = field.chars().count();
            if self.settings_edit_cursor < len {
                self.settings_edit_cursor += 1;
            }
        }
    }

    pub fn insert_list_char(&mut self, c: char) {
        if let Some(idx) = self.list_edit_index {
            let cursor = self.list_input_cursor;
            let category = self.settings_categories[self.active_settings_category];
            if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                let mut chars: Vec<char> = toggle.list_input.chars().collect();
                chars.insert(cursor, c);
                toggle.list_input = chars.into_iter().collect();
            }
            self.list_input_cursor += 1;
        }
    }

    pub fn delete_list_char(&mut self) {
        if self.list_input_cursor > 0 {
            if let Some(idx) = self.list_edit_index {
                let cursor = self.list_input_cursor - 1;
                let category = self.settings_categories[self.active_settings_category];
                if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                    let mut chars: Vec<char> = toggle.list_input.chars().collect();
                    if cursor < chars.len() {
                        chars.remove(cursor);
                        toggle.list_input = chars.into_iter().collect();
                    }
                }
                self.list_input_cursor -= 1;
            }
        }
    }

    pub fn move_list_cursor_left(&mut self) {
        if self.list_input_cursor > 0 {
            self.list_input_cursor -= 1;
        }
    }

    pub fn move_list_cursor_right(&mut self) {
        let cursor = self.list_input_cursor;
        if let Some(idx) = self.list_edit_index {
            let category = self.settings_categories[self.active_settings_category];
            if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                let len = toggle.list_input.chars().count();
                if cursor < len {
                    self.list_input_cursor += 1;
                }
            }
        }
    }

    pub fn save_list_input(&mut self) {
        if let Some(idx) = self.list_edit_index {
            let category = self.settings_categories[self.active_settings_category];
            if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                let new_list: Vec<String> = toggle
                    .list_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                toggle.list = new_list;
                toggle.list_input_active = false;
                toggle.list_input.clear();
                self.list_edit_index = None;
                self.list_input_cursor = 0;
            }
        }
    }

    pub fn cancel_list_input(&mut self) {
        if let Some(idx) = self.list_edit_index {
            let category = self.settings_categories[self.active_settings_category];
            if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                toggle.list_input_active = false;
                toggle.list_input.clear();
                self.list_edit_index = None;
                self.list_input_cursor = 0;
            }
        }
    }

    pub fn set_status(&mut self, msg: &str, _color: Color) {
        self.status_message = Some((msg.to_string(), Duration::from_secs(3)));
    }

    pub fn update_status(&mut self) {
        if let Some((_, duration)) = &mut self.status_message {
            if duration.as_millis() > 0 {
                *duration = Duration::from_millis(duration.as_millis().saturating_sub(100) as u64);
            } else {
                self.status_message = None;
            }
        }
    }

    pub fn is_toggle_enabled(&self, category: SettingsCategory, index: usize) -> bool {
        match category {
            SettingsCategory::Git => match index {
                0 => matches!(
                    self.config.init_git.mode,
                    ToggleMode::Yes | ToggleMode::YesSome
                ),
                1 => matches!(
                    self.config.create_local_gitignore.mode,
                    ToggleMode::Yes | ToggleMode::YesSome
                ),
                _ => false,
            },
            SettingsCategory::Actions => match index {
                0 => matches!(
                    self.config.open_terminal.mode,
                    ToggleMode::Yes | ToggleMode::YesSome
                ),
                1 => matches!(
                    self.config.open_ide.mode,
                    ToggleMode::Yes | ToggleMode::YesSome
                ),
                _ => false,
            },
            _ => false,
        }
    }

    pub fn get_toggle_recipes(&self, category: SettingsCategory, index: usize) -> Vec<String> {
        match category {
            SettingsCategory::Git => match index {
                0 => self.config.init_git.list.clone(),
                1 => self.config.create_local_gitignore.list.clone(),
                _ => vec![],
            },
            SettingsCategory::Actions => match index {
                0 => self.config.open_terminal.list.clone(),
                1 => self.config.open_ide.list.clone(),
                _ => vec![],
            },
            _ => vec![],
        }
    }

    pub fn is_list_editing(&self, index: usize) -> bool {
        self.list_edit_index == Some(index)
    }

    pub fn get_list_input(&self) -> String {
        if let Some(idx) = self.list_edit_index {
            let category = self.settings_categories[self.active_settings_category];
            match category {
                SettingsCategory::Git => match idx {
                    0 => return self.config.init_git.list_input.clone(),
                    1 => return self.config.create_local_gitignore.list_input.clone(),
                    _ => {}
                },
                SettingsCategory::Actions => match idx {
                    0 => return self.config.open_terminal.list_input.clone(),
                    1 => return self.config.open_ide.list_input.clone(),
                    _ => {}
                },
                _ => {}
            }
        }
        String::new()
    }

    pub fn get_list_input_cursor(&self) -> usize {
        self.list_input_cursor
    }
}
