use crate::exstract::{extract_slug, fetch_problem};
use crate::generator::create_project;
use crate::structs::{
    AllSettings, AppConfig, ConfigFileCreation, LeetCodeProblem, Project, ToggleMode, ToggleWithList
};
use crate::utils::{get_all_config, get_all_projects};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CreationStage {
    ExtractingSlug,
    FetchingProblem,
    CreatingProject,
    RunningCommands,
    Finalizing,
}

impl CreationStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            CreationStage::ExtractingSlug => "Extracting slug from URL...",
            CreationStage::FetchingProblem => "Getting task information...",
            CreationStage::CreatingProject => "Creating project structure...",
            CreationStage::RunningCommands => "Executing setup commands...",
            CreationStage::Finalizing => "Completing...",
        }
    }

    pub fn progress_weight(&self) -> f64 {
        match self {
            CreationStage::ExtractingSlug => 0.05,
            CreationStage::FetchingProblem => 0.25,
            CreationStage::CreatingProject => 0.40,
            CreationStage::RunningCommands => 0.20,
            CreationStage::Finalizing => 0.10,
        }
    }
}

pub enum AppState {
    Input,
    Creating {
        current_stage: CreationStage,
        stage_progress: f64,
        total_progress: f64,
        status_message: String,
        problem:Option<LeetCodeProblem>
    },
    Done,
}

pub struct App {
    pub main_tabs: Vec<&'static str>,
    pub active_main_tab: usize,
    pub url_input: String,
    pub cursor_position: usize,
    pub input_active: bool,
    pub projects: Vec<Project>,
    pub projects_scroll: usize,
    pub selected_recipe: Option<ConfigFileCreation>,
    pub recipes: Vec<ConfigFileCreation>,
    pub settings: AllSettings,
    pub settings_categories: Vec<SettingsCategory>,
    pub active_settings_category: usize,
    pub settings_cursor: usize,
    pub settings_edit_index: Option<(usize, usize)>,
    pub settings_edit_cursor: usize,
    pub list_edit_index: Option<usize>,
    pub list_input_cursor: usize,
    pub state: AppState,
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
            Err(_) => {
                let cfg = AppConfig::default();
                _ = confy::store("tui-solution-create", None, &cfg);
                cfg
            }
        };
        let settings = AllSettings::from_config(&config);
        Self {
            main_tabs: vec!["Recipes", "Settings"],
            active_main_tab: 0,
            url_input: String::new(),
            cursor_position: 0,
            input_active: false,
            selected_recipe: None,
            projects: get_all_projects(&settings.solutions_url),
            recipes: get_all_config(&settings.recipes_url),
            projects_scroll: 0,
            settings: settings,
            settings_categories: SettingsCategory::all(),
            active_settings_category: 0,
            settings_cursor: 0,
            settings_edit_index: None,
            settings_edit_cursor: 0,
            list_edit_index: None,
            list_input_cursor: 0,
            state: AppState::Input,
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

    pub fn get_update_recipes(&mut self){
        self.recipes = get_all_config(&self.settings.recipes_url);
    }
    pub fn get_update_projects(&mut self){
        self.projects = get_all_projects(&self.settings.solutions_url);
    }

    pub fn open_language_popup(&mut self) {
        self.language_popup_open = true;
        self.language_popup_in_modal = false;
        self.language_search.clear();
        self.language_search_cursor = 0;
        self.language_filtered_index = 0;
        self.get_update_recipes()
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
        if let Some((orig_idx, c)) = filtered.get(self.language_filtered_index) {
            let idx = *orig_idx;
            if idx < self.recipes.len() {
                self.selected_recipe = Some(c.clone());
            }
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
        for config in get_all_config(&self.settings.recipes_url) {
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

    pub fn get_filtered_languages(&self) -> Vec<(usize, ConfigFileCreation)> {
        if self.language_search.is_empty() {
            self.recipes
                .iter()
                .enumerate()
                .map(|(i, lang)| (i, lang.clone()))
                .collect()
        } else {
            self.recipes
                .iter()
                .enumerate()
                .filter(|(_, lang)| {
                    lang.name
                        .to_lowercase()
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
        self.settings.init_git.list_input_active = false;
        self.settings.create_local_gitignore.list_input_active = false;
        self.settings.open_terminal.list_input_active = false;
        self.settings.open_ide.list_input_active = false;
    }

    pub fn get_toggle_for_setting(
        &mut self,
        category: SettingsCategory,
        index: usize,
    ) -> Option<&mut ToggleWithList> {
        match category {
            SettingsCategory::Git => match index {
                0 => Some(&mut self.settings.init_git),
                1 => Some(&mut self.settings.create_local_gitignore),
                _ => None,
            },
            SettingsCategory::Actions => match index {
                0 => Some(&mut self.settings.open_terminal),
                1 => Some(&mut self.settings.open_ide),
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
                    self.settings.recipes_url = if self.settings.recipes_url.is_empty() {
                        "recipes".to_string()
                    } else {
                        String::new()
                    };
                }
                1 => {
                    self.settings.solutions_url = if self.settings.solutions_url.is_empty() {
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

    pub fn start_creation(&mut self) {
        self.state = AppState::Creating {
            current_stage: CreationStage::ExtractingSlug,
            stage_progress: 0.0,
            total_progress: 0.0,
            status_message: "Starting project creation...".to_string(),
            problem: None
        };
    }

    pub async fn update_creation(&mut self) -> Result<bool, String> {
        let slug_opt = extract_slug(&self.url_input);
        let selected_config = match &self.selected_recipe {
            Some(c) => c,
            None => return Err("Recipes not selected".to_string()),
        };
        let slug = slug_opt.clone().ok_or("Invalid URL")?;
        if let AppState::Creating {
            current_stage,
            stage_progress,
            total_progress,
            status_message,
            problem
        } = &mut self.state
        {
            match current_stage {
                CreationStage::ExtractingSlug => {
                    *stage_progress += 0.2;
                    *status_message = format!("Slug: {}", slug);
                    if *stage_progress >= 1.0 {
                        *current_stage = CreationStage::FetchingProblem;
                        *stage_progress = 0.0;
                    }
                }
                CreationStage::FetchingProblem => {
                    *status_message = "Loading task information...".to_string();
                    *problem = match fetch_problem(&slug).await {
                        Ok(problem) => {
                            *status_message = format!("Task: {}", problem.title);
                            *stage_progress = 1.0;
                            *current_stage = CreationStage::CreatingProject;
                            *stage_progress = 0.0;
                            Some(problem)
                        }
                        Err(e) => {
                            return Err(format!("Error fetching task: {}", e));
                        }
                    }
                }
                CreationStage::CreatingProject => {
                    *status_message = format!("Creating project: {}", slug);

                    if *stage_progress < 0.3 {
                        *status_message = "Creating directories...".to_string();
                        *stage_progress = 0.3;
                    } else if *stage_progress < 0.6 {
                        *status_message = "Creating files...".to_string();
                        *stage_progress = 0.6;
                    } else if *stage_progress < 1.0 {
                        match problem {
                            Some(p) => {
                                create_project(&slug, &selected_config, &self.settings.to_config(), &p)
                            .map_err(|e| format!("Error creating project: {}", e))?;
                        *stage_progress = 1.0;
                        *current_stage = CreationStage::RunningCommands;
                        *stage_progress = 0.0;
                            },
                            None => return Err("Filed get problem".to_string()),
                        }
                        
                    }
                }
                CreationStage::RunningCommands => {
                    *status_message = "Executing setup commands...".to_string();
                    *stage_progress = 1.0;
                    *current_stage = CreationStage::Finalizing;
                    *stage_progress = 0.0;
                }
                CreationStage::Finalizing => {
                    *status_message = "Finalizing...".to_string();
                    *stage_progress = 1.0;

                    self.state = AppState::Done;
                    self.set_status(&format!("Project '{}' created!", slug));
                    self.clear_url_input();
                    return Ok(true);
                }
            }

            *total_progress = calculate_total_progress(*current_stage, *stage_progress);
        }
        Ok(false)
    }

    pub fn reset_creation(&mut self) {
        self.state = AppState::Input;
    }

    pub fn clear_url_input(&mut self) {
        self.url_input.clear();
        self.cursor_position = 0;
    }
}

fn calculate_total_progress(current: CreationStage, stage_prog: f64) -> f64 {
    let mut total = 0.0;
    for stage in [
        CreationStage::ExtractingSlug,
        CreationStage::FetchingProblem,
        CreationStage::CreatingProject,
        CreationStage::RunningCommands,
        CreationStage::Finalizing,
    ] {
        if stage == current {
            total += stage_prog * stage.progress_weight();
            break;
        } else {
            total += stage.progress_weight();
        }
    }
    total.min(1.0)
}

impl App {
    pub fn start_editing_setting(&mut self) {
        let category = self.settings_categories[self.active_settings_category];
        if category == SettingsCategory::General {
            match self.settings_cursor {
                0 => {
                    self.settings_edit_index = Some((0, self.settings_cursor));
                    self.settings_edit_cursor = self.settings.recipes_url.len();
                }
                1 => {
                    self.settings_edit_index = Some((1, self.settings_cursor));
                    self.settings_edit_cursor = self.settings.solutions_url.len();
                }
                _ => {}
            }
        }
    }

    pub fn stop_editing_setting(&mut self) {
        self.settings_edit_index = None;
    }

    pub fn save_settings(&mut self) {
        let config = self.settings.to_config();
        let _: Result<(), _> = confy::store("tui-solution-create", None, &config);
        self.set_status("Settings saved");
    }

    fn get_editable_field(&self) -> Option<&String> {
        match self.settings_edit_index {
            Some((0, _)) => Some(&self.settings.recipes_url),
            Some((1, _)) => Some(&self.settings.solutions_url),
            _ => None,
        }
    }

    fn get_editable_field_mut(&mut self) -> Option<&mut String> {
        match self.settings_edit_index {
            Some((0, _)) => Some(&mut self.settings.recipes_url),
            Some((1, _)) => Some(&mut self.settings.solutions_url),
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

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), Duration::from_secs(2)));
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
                    self.settings.init_git.mode,
                    ToggleMode::Yes | ToggleMode::YesSome
                ),
                1 => matches!(
                    self.settings.create_local_gitignore.mode,
                    ToggleMode::Yes | ToggleMode::YesSome
                ),
                _ => false,
            },
            SettingsCategory::Actions => match index {
                0 => matches!(
                    self.settings.open_terminal.mode,
                    ToggleMode::Yes | ToggleMode::YesSome
                ),
                1 => matches!(
                    self.settings.open_ide.mode,
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
                0 => self.settings.init_git.list.clone(),
                1 => self.settings.create_local_gitignore.list.clone(),
                _ => vec![],
            },
            SettingsCategory::Actions => match index {
                0 => self.settings.open_terminal.list.clone(),
                1 => self.settings.open_ide.list.clone(),
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
                    0 => return self.settings.init_git.list_input.clone(),
                    1 => return self.settings.create_local_gitignore.list_input.clone(),
                    _ => {}
                },
                SettingsCategory::Actions => match idx {
                    0 => return self.settings.open_terminal.list_input.clone(),
                    1 => return self.settings.open_ide.list_input.clone(),
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
