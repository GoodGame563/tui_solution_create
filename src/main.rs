mod structs;
mod generator;
use structs::{AppConfig, ToggleMode, ToggleWithList};

use std::{io, fs};
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Gauge, Clear, Padding},
    Frame, Terminal,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusSection {
    UrlInput,
    Language,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsCategory {
    General,
    Git,
    Actions,
}

impl SettingsCategory {
    fn all() -> Vec<SettingsCategory> {
        vec![SettingsCategory::General, SettingsCategory::Git, SettingsCategory::Actions]
    }

    fn title(&self) -> &'static str {
        match self {
            SettingsCategory::General => "General",
            SettingsCategory::Git => "Git",
            SettingsCategory::Actions => "Actions",
        }
    }
}

struct App {
    main_tabs: Vec<&'static str>,
    active_main_tab: usize,
    url_input: String,
    cursor_position: usize,
    input_mode: bool,
    languages: Vec<String>,
    selected_language_index: usize,
    projects: Vec<(String, String)>,
    projects_scroll: usize,
    config: AppConfig,
    settings_categories: Vec<SettingsCategory>,
    active_settings_category: usize,
    settings_cursor: usize,
    settings_edit_index: Option<(usize, usize)>,
    settings_edit_cursor: usize,
    list_edit_index: Option<usize>,
    list_input_cursor: usize,
    state: AppState,
    creation_stages: Vec<&'static str>,
    focus_section: FocusSection,
    language_popup_open: bool,
    language_search: String,
    language_search_cursor: usize,
    status_message: Option<(String, Duration)>,
}

enum AppState {
    Input,
    Creating { progress: f64, stage: usize },
    Done,
}

impl App {
    fn new() -> Self {
        let languages = Self::load_languages();
        Self {
            main_tabs: vec!["Recipes", "Settings"],
            active_main_tab: 0,
            url_input: String::new(),
            cursor_position: 0,
            input_mode: false,
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
            config: AppConfig::default(),
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
            focus_section: FocusSection::UrlInput,
            language_popup_open: false,
            language_search: String::new(),
            language_search_cursor: 0,
            status_message: None,
        }
    }

    fn load_languages() -> Vec<String> {
        let mut langs = Vec::new();
        if let Ok(entries) = fs::read_dir("recipes") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        langs.push(stem.to_string());
                    }
                }
            }
        }
        if langs.is_empty() {
            langs.push("python".to_string());
        }
        langs.sort();
        langs
    }

    fn selected_language(&self) -> String {
        self.languages.get(self.selected_language_index)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn cycle_language(&mut self) {
        if !self.languages.is_empty() {
            self.selected_language_index = (self.selected_language_index + 1) % self.languages.len();
        }
    }

    fn open_language_popup(&mut self) {
        self.language_popup_open = true;
        self.language_search.clear();
        self.language_search_cursor = 0;
    }

    fn close_language_popup(&mut self) {
        self.language_popup_open = false;
    }

    fn confirm_language_selection(&mut self) {
        self.language_popup_open = false;
        self.language_search.clear();
        self.language_search_cursor = 0;
    }

    fn get_filtered_languages(&self) -> Vec<(usize, String)> {
        if self.language_search.is_empty() {
            self.languages.iter()
                .enumerate()
                .map(|(i, lang)| (i, lang.clone()))
                .collect()
        } else {
            self.languages.iter()
                .enumerate()
                .filter(|(_, lang)| lang.to_lowercase().contains(&self.language_search.to_lowercase()))
                .map(|(i, lang)| (i, lang.clone()))
                .collect()
        }
    }

    fn move_language_selection(&mut self, direction: i8) {
        if direction > 0 {
            self.selected_language_index = (self.selected_language_index + 1) % self.languages.len();
        } else {
            self.selected_language_index = if self.selected_language_index == 0 {
                self.languages.len() - 1
            } else {
                self.selected_language_index - 1
            };
        }
    }

    fn insert_language_search_char(&mut self, c: char) {
        self.language_search.insert(self.language_search_cursor, c);
        self.language_search_cursor += 1;
    }

    fn delete_language_search_char(&mut self) {
        if self.language_search_cursor > 0 {
            self.language_search.remove(self.language_search_cursor - 1);
            self.language_search_cursor -= 1;
        }
    }

    fn next_main_tab(&mut self) {
        self.active_main_tab = (self.active_main_tab + 1) % self.main_tabs.len();
    }

    fn previous_main_tab(&mut self) {
        if self.active_main_tab > 0 {
            self.active_main_tab -= 1;
        } else {
            self.active_main_tab = self.main_tabs.len() - 1;
        }
    }

    fn next_settings_category(&mut self) {
        self.active_settings_category = (self.active_settings_category + 1) % self.settings_categories.len();
        self.settings_cursor = 0;
        self.list_edit_index = None;
    }

    fn previous_settings_category(&mut self) {
        if self.active_settings_category > 0 {
            self.active_settings_category -= 1;
        } else {
            self.active_settings_category = self.settings_categories.len() - 1;
        }
        self.settings_cursor = 0;
        self.list_edit_index = None;
    }

    fn enter_input_mode(&mut self) {
        self.input_mode = true;
    }

    fn leave_input_mode(&mut self) {
        self.input_mode = false;
    }

    fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.url_input.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    fn insert_char(&mut self, c: char) {
        self.url_input.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.url_input.len() {
            self.cursor_position += 1;
        }
    }

    fn paste_from_clipboard(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                for c in text.chars() {
                    self.url_input.insert(self.cursor_position, c);
                    self.cursor_position += 1;
                }
            }
        }
    }

    fn scroll_projects(&mut self, direction: i8) {
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

    fn get_settings_items_count(&self) -> usize {
        match self.settings_categories[self.active_settings_category] {
            SettingsCategory::General => 2,
            SettingsCategory::Git => 2,
            SettingsCategory::Actions => 2,
        }
    }

    fn move_settings_cursor(&mut self, direction: i8) {
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

    fn get_toggle_for_setting(&mut self, category: SettingsCategory, index: usize) -> Option<&mut ToggleWithList> {
        match category {
            SettingsCategory::Git => {
                match index {
                    0 => Some(&mut self.config.init_git),
                    1 => Some(&mut self.config.create_local_gitignore),
                    _ => None,
                }
            }
            SettingsCategory::Actions => {
                match index {
                    0 => Some(&mut self.config.open_terminal),
                    1 => Some(&mut self.config.open_ide),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn toggle_setting(&mut self) {
        self.deactivate_all_list_inputs();
        let category = self.settings_categories[self.active_settings_category];
        let settings_cursor = self.settings_cursor;

        match category {
            SettingsCategory::General => {
                match settings_cursor {
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
                }
            }
            SettingsCategory::Git | SettingsCategory::Actions => {
                let needs_list_input = if let Some(toggle) = self.get_toggle_for_setting(category, settings_cursor) {
                    toggle.mode = match toggle.mode {
                        ToggleMode::No => ToggleMode::Yes,
                        ToggleMode::Yes => ToggleMode::YesSome,
                        ToggleMode::YesSome => ToggleMode::No,
                    };
                    toggle.mode == ToggleMode::YesSome
                } else {
                    false
                };

                if needs_list_input {
                    self.list_edit_index = Some(settings_cursor);
                    self.list_input_cursor = 0;
                    if let Some(toggle) = self.get_toggle_for_setting(category, settings_cursor) {
                        toggle.list_input_active = true;
                        toggle.list_input.clear();
                    }
                } else {
                    if let Some(toggle) = self.get_toggle_for_setting(category, settings_cursor) {
                        toggle.list_input_active = false;
                    }
                    self.list_edit_index = None;
                }
            }
        }
    }

    fn submit_url(&mut self) {
        if !self.url_input.is_empty() {
            self.state = AppState::Creating { progress: 0.0, stage: 0 };
            self.input_mode = false;
        } else {
            self.set_status("Enter project name", Color::Yellow);
        }
    }

    fn update_creation(&mut self) {
        if let AppState::Creating { progress, stage } = &mut self.state {
            *progress += 0.02;
            if *progress >= 1.0 {
                *progress = 0.0;
                *stage += 1;
                if *stage >= self.creation_stages.len() {
                    self.state = AppState::Done;
                    self.projects.insert(0, (self.selected_language(), self.url_input.clone()));
                    self.set_status(&format!("Project '{}' created!", self.url_input), Color::Green);
                    self.url_input.clear();
                    self.cursor_position = 0;
                }
            }
        }
    }

    fn reset_creation(&mut self) {
        self.state = AppState::Input;
    }

    fn start_editing_setting(&mut self) {
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

    fn stop_editing_setting(&mut self) {
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

    fn insert_setting_char(&mut self, c: char) {
        let cursor = self.settings_edit_cursor;
        if let Some(field) = self.get_editable_field_mut() {
            field.insert(cursor, c);
            self.settings_edit_cursor += 1;
        }
    }

    fn delete_setting_char(&mut self) {
        if self.settings_edit_cursor > 0 {
            let cursor = self.settings_edit_cursor - 1;
            if let Some(field) = self.get_editable_field_mut() {
                field.remove(cursor);
                self.settings_edit_cursor -= 1;
            }
        }
    }

    fn move_setting_cursor_left(&mut self) {
        if self.settings_edit_cursor > 0 {
            self.settings_edit_cursor -= 1;
        }
    }

    fn move_setting_cursor_right(&mut self) {
        if let Some(field) = self.get_editable_field() {
            if self.settings_edit_cursor < field.len() {
                self.settings_edit_cursor += 1;
            }
        }
    }

    fn insert_list_char(&mut self, c: char) {
        if let Some(idx) = self.list_edit_index {
            let cursor = self.list_input_cursor;
            let category = self.settings_categories[self.active_settings_category];
            if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                toggle.list_input.insert(cursor, c);
            }
            self.list_input_cursor += 1;
        }
    }

    fn delete_list_char(&mut self) {
        if self.list_input_cursor > 0 {
            if let Some(idx) = self.list_edit_index {
                let cursor = self.list_input_cursor - 1;
                let category = self.settings_categories[self.active_settings_category];
                if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                    toggle.list_input.remove(cursor);
                }
                self.list_input_cursor -= 1;
            }
        }
    }

    fn move_list_cursor_left(&mut self) {
        if self.list_input_cursor > 0 {
            self.list_input_cursor -= 1;
        }
    }

    fn move_list_cursor_right(&mut self) {
        let cursor = self.list_input_cursor;
        if let Some(idx) = self.list_edit_index {
            let category = self.settings_categories[self.active_settings_category];
            if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                if cursor < toggle.list_input.len() {
                    self.list_input_cursor += 1;
                }
            }
        }
    }

    fn save_list_input(&mut self) {
        if let Some(idx) = self.list_edit_index {
            let category = self.settings_categories[self.active_settings_category];
            if let Some(toggle) = self.get_toggle_for_setting(category, idx) {
                let new_list: Vec<String> = toggle.list_input
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

    fn cancel_list_input(&mut self) {
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

    fn set_status(&mut self, msg: &str, _color: Color) {
        self.status_message = Some((msg.to_string(), Duration::from_secs(3)));
    }

    fn update_status(&mut self) {
        if let Some((_, duration)) = &mut self.status_message {
            if duration.as_millis() > 0 {
                *duration = Duration::from_millis(duration.as_millis().saturating_sub(100) as u64);
            } else {
                self.status_message = None;
            }
        }
    }

    fn is_toggle_enabled(&self, category: SettingsCategory, index: usize) -> bool {
        match category {
            SettingsCategory::Git => {
                match index {
                    0 => matches!(self.config.init_git.mode, ToggleMode::Yes | ToggleMode::YesSome),
                    1 => matches!(self.config.create_local_gitignore.mode, ToggleMode::Yes | ToggleMode::YesSome),
                    _ => false,
                }
            }
            SettingsCategory::Actions => {
                match index {
                    0 => matches!(self.config.open_terminal.mode, ToggleMode::Yes | ToggleMode::YesSome),
                    1 => matches!(self.config.open_ide.mode, ToggleMode::Yes | ToggleMode::YesSome),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn get_toggle_templates(&self, category: SettingsCategory, index: usize) -> Vec<String> {
        match category {
            SettingsCategory::Git => {
                match index {
                    0 => self.config.init_git.list.clone(),
                    1 => self.config.create_local_gitignore.list.clone(),
                    _ => vec![],
                }
            }
            SettingsCategory::Actions => {
                match index {
                    0 => self.config.open_terminal.list.clone(),
                    1 => self.config.open_ide.list.clone(),
                    _ => vec![],
                }
            }
            _ => vec![],
        }
    }

    fn is_list_editing(&self, index: usize) -> bool {
        self.list_edit_index == Some(index)
    }

    fn get_list_input(&self) -> String {
        if let Some(idx) = self.list_edit_index {
            let category = self.settings_categories[self.active_settings_category];
            match category {
                SettingsCategory::Git => {
                    match idx {
                        0 => return self.config.init_git.list_input.clone(),
                        1 => return self.config.create_local_gitignore.list_input.clone(),
                        _ => {}
                    }
                }
                SettingsCategory::Actions => {
                    match idx {
                        0 => return self.config.open_terminal.list_input.clone(),
                        1 => return self.config.open_ide.list_input.clone(),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        String::new()
    }

    fn get_list_input_cursor(&self) -> usize {
        self.list_input_cursor
    }
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui(f, app)).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.state {
                        AppState::Input => {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
                                KeyCode::Tab => {
                                    if !app.input_mode && app.active_main_tab == 0 {
                                        app.next_main_tab();
                                    } else if app.active_main_tab == 1 {
                                        app.previous_main_tab();
                                    }
                                }
                                KeyCode::BackTab => {
                                    if app.active_main_tab == 1 {
                                        app.previous_main_tab();
                                    }
                                }
                                KeyCode::Left => {
                                    if app.language_popup_open {
                                        app.move_language_selection(-1);
                                    } else if app.input_mode && app.active_main_tab == 0 {
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
                                    if app.language_popup_open {
                                        app.move_language_selection(1);
                                    } else if app.input_mode && app.active_main_tab == 0 {
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
                                    if app.language_popup_open {
                                        app.move_language_selection(-1);
                                    } else if app.active_main_tab == 1 {
                                        if app.settings_edit_index.is_none() && app.list_edit_index.is_none() {
                                            app.move_settings_cursor(-1);
                                        }
                                    } else if app.active_main_tab == 0 {
                                        app.scroll_projects(-1);
                                    }
                                }
                                KeyCode::Down => {
                                    if app.language_popup_open {
                                        app.move_language_selection(1);
                                    } else if app.active_main_tab == 1 {
                                        if app.settings_edit_index.is_none() && app.list_edit_index.is_none() {
                                            app.move_settings_cursor(1);
                                        }
                                    } else if app.active_main_tab == 0 {
                                        app.scroll_projects(1);
                                    }
                                }
                                KeyCode::Enter => {
                                    if app.language_popup_open {
                                        app.confirm_language_selection();
                                    } else if !app.input_mode && app.active_main_tab == 0 {
                                        app.enter_input_mode();
                                    } else if app.input_mode && app.active_main_tab == 0 {
                                        app.submit_url();
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
                                    if app.language_popup_open {
                                        app.close_language_popup();
                                    } else if app.input_mode {
                                        app.leave_input_mode();
                                    } else if app.active_main_tab == 1 {
                                        if app.settings_edit_index.is_some() {
                                            app.stop_editing_setting();
                                        } else if app.list_edit_index.is_some() {
                                            app.cancel_list_input();
                                        }
                                    }
                                }
                                KeyCode::Backspace => {
                                    if app.language_popup_open {
                                        app.delete_language_search_char();
                                    } else if app.input_mode && app.active_main_tab == 0 {
                                        app.delete_char();
                                    } else if app.active_main_tab == 1 {
                                        if app.settings_edit_index.is_some() {
                                            app.delete_setting_char();
                                        } else if app.list_edit_index.is_some() {
                                            app.delete_list_char();
                                        }
                                    }
                                }
                                KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    if app.input_mode && app.active_main_tab == 0 {
                                        app.paste_from_clipboard();
                                    }
                                }
                                KeyCode::Char('ё') | KeyCode::Char('`') => {
                                    if app.active_main_tab == 0 && !app.input_mode {
                                        app.open_language_popup();
                                    } else if app.input_mode && app.active_main_tab == 0 {
                                        app.cycle_language();
                                    }
                                }
                                KeyCode::Char(' ') => {
                                    if app.active_main_tab == 1 && app.settings_edit_index.is_none() && app.list_edit_index.is_none() {
                                        app.toggle_setting();
                                    }
                                }
                                KeyCode::Char(c) => {
                                    if app.language_popup_open {
                                        app.insert_language_search_char(c);
                                    } else if app.input_mode && app.active_main_tab == 0 {
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
                        AppState::Creating { .. } => {
                            if key.code == KeyCode::Char('q') {
                                return Ok(());
                            }
                        }
                        AppState::Done => {
                            if key.code == KeyCode::Enter {
                                app.reset_creation();
                            } else if key.code == KeyCode::Char('q') {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            if let AppState::Creating { .. } = app.state {
                app.update_creation();
            }
            app.update_status();
            last_tick = std::time::Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(size);

    render_tabs(f, app, main_chunks[0]);

    if app.active_main_tab == 0 {
        render_recipes_tab(f, app, main_chunks[1]);
    } else {
        render_settings_tab(f, app, main_chunks[1]);
    }

    render_status_bar(f, app, main_chunks[2]);

    if app.language_popup_open {
        render_language_popup(f, app);
    }
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app
        .main_tabs
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if app.active_main_tab == i {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(0, 150, 0))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(Span::styled(*t, style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Line::from(vec![
                    Span::styled("solution_create", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]))
                .title_alignment(Alignment::Center),
        )
        .select(app.active_main_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_text = if let AppState::Creating { stage, .. } = &app.state {
        Line::from(vec![
            Span::styled(app.creation_stages[*stage], Style::default().fg(Color::Yellow)),
        ])
    } else if let AppState::Done = &app.state {
        Line::from(vec![
            Span::styled("Project created! Press Enter to continue", Style::default().fg(Color::Green)),
        ])
    } else if let Some((msg, _)) = &app.status_message {
        Line::from(vec![
            Span::raw(msg.clone()),
        ])
    } else if app.active_main_tab == 0 {
        Line::from(vec![
            Span::styled("Ctrl+V", Style::default().fg(Color::Cyan)),
            Span::raw(" Paste  "),
            Span::styled("ё", Style::default().fg(Color::Cyan)),
            Span::raw(" Language  "),
            Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
            Span::raw(" Scroll  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" Create  "),
            Span::styled("Tab", Style::default().fg(Color::Magenta)),
            Span::raw(" Settings  "),
            Span::styled("q", Style::default().fg(Color::Red)),
            Span::raw(" Quit"),
        ])
    } else {
        let help_text = if app.list_edit_index.is_some() {
            "Enter Save  Esc Cancel"
        } else if app.settings_edit_index.is_some() {
            "Enter Save  Esc Cancel"
        } else {
            "Enter/Space Toggle"
        };
        Line::from(vec![
            Span::styled(help_text, Style::default().fg(Color::Green)),
            Span::raw("  "),
            Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
            Span::raw(" Navigate  "),
            Span::styled("←/→", Style::default().fg(Color::Magenta)),
            Span::raw(" Categories  "),
            Span::styled("Tab", Style::default().fg(Color::Magenta)),
            Span::raw(" Recipes  "),
            Span::styled("q", Style::default().fg(Color::Red)),
            Span::raw(" Quit"),
        ])
    };

    let status = Paragraph::new(status_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Status ")
                .title_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);

    f.render_widget(status, area);
}

fn render_recipes_tab(f: &mut Frame, app: &App, area: Rect) {
    match &app.state {
        AppState::Input | AppState::Done => {
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(5),
                ])
                .split(area);

            let input_style = if app.input_mode && app.focus_section == FocusSection::UrlInput {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            let input_widget = Paragraph::new(app.url_input.as_str())
                .style(input_style)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Project Name ")
                        .border_style(Style::default().fg(if app.input_mode { Color::Yellow } else { Color::DarkGray }))
                        .padding(Padding::horizontal(1)),
                );

            f.render_widget(input_widget, left_chunks[0]);

            if app.input_mode {
                f.set_cursor_position((
                    left_chunks[0].x + app.cursor_position as u16 + 1,
                    left_chunks[0].y + 1,
                ));
            }

            let selected_lang = app.selected_language();
            let lang_color = get_language_color(&selected_lang);

            let lang_style = if app.focus_section == FocusSection::Language && !app.input_mode {
                Style::default().fg(lang_color).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(lang_color).add_modifier(Modifier::BOLD)
            };

            let lang_widget = Paragraph::new(selected_lang.as_str())
                .style(lang_style)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Language (ё to change) ")
                        .border_style(Style::default().fg(Color::DarkGray))
                        .padding(Padding::horizontal(1)),
                )
                .alignment(Alignment::Center);

            f.render_widget(lang_widget, left_chunks[1]);

            let visible_projects = area.height.saturating_sub(6) as usize;
            let visible_projects = if visible_projects < 1 { 1 } else { visible_projects };
            let start = app.projects_scroll;

            let projects: Vec<ListItem> = app
                .projects
                .iter()
                .skip(start)
                .take(visible_projects)
                .map(|(lang, name)| {
                    let lang_color = get_language_color(lang);
                    let content = Line::from(vec![
                        Span::styled(
                            format!("{:<12}", lang),
                            Style::default().fg(lang_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" > ", Style::default().fg(Color::DarkGray)),
                        Span::raw(name),
                    ]);
                    ListItem::new(content)
                })
                .collect();

            let scroll_info = if app.projects.len() > visible_projects {
                format!(" [{}/{}]", start + 1, app.projects.len())
            } else {
                String::new()
            };

            let projects_list = List::new(projects)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Recent Projects{} ", scroll_info))
                        .border_style(Style::default().fg(Color::DarkGray))
                        .padding(Padding::horizontal(1)),
                )
                .style(Style::default().fg(Color::White));

            f.render_widget(projects_list, left_chunks[2]);
        }
        AppState::Creating { progress, stage } => {
            render_creation_screen(f, app, area, *progress, *stage);
        }
    }
}

fn get_language_color(lang: &str) -> Color {
    match lang.to_lowercase().as_str() {
        "go" => Color::Rgb(0, 200, 200),
        "rust" => Color::Rgb(255, 100, 100),
        "python" => Color::Rgb(100, 150, 255),
        "javascript" | "js" => Color::Rgb(255, 200, 50),
        "typescript" | "ts" => Color::Rgb(50, 150, 255),
        _ => Color::White,
    }
}

fn render_creation_screen(f: &mut Frame, app: &App, area: Rect, progress: f64, stage: usize) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("Creating Solution...")
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .padding(Padding::horizontal(1)),
        );

    f.render_widget(title, chunks[0]);

    let stages: Vec<ListItem> = app
        .creation_stages
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == stage {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if i < stage {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let prefix = if i < stage { "[OK]" } else if i == stage { "[..]" } else { "[  ]" };
            ListItem::new(Line::from(Span::styled(format!(" {} {}", prefix, s), style)))
        })
        .collect();

    let stages_list = List::new(stages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .padding(Padding::horizontal(1)),
        );

    f.render_widget(stages_list, chunks[1]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Green))
        .percent((progress * 100.0) as u16)
        .label(Span::styled(format!("{:.0}%", progress * 100.0), Style::default().fg(Color::White)));

    f.render_widget(gauge, chunks[2]);
}

fn render_settings_tab(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
        ])
        .split(area);

    let category_tabs: Vec<Line> = app.settings_categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let style = if i == app.active_settings_category {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(0, 100, 150))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(Span::styled(cat.title(), style))
        })
        .collect();

    let category_tabs_widget = Tabs::new(category_tabs)
        .select(app.active_settings_category)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(0, 100, 150))
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(category_tabs_widget, chunks[0]);

    let settings_items = get_settings_for_category(app);

    let items: Vec<ListItem> = settings_items
        .iter()
        .enumerate()
        .map(|(i, (label, description, is_enabled, has_templates, is_editing, edit_value, templates, is_list_editing, list_input))| {
            let is_selected = i == app.settings_cursor;

            let checkbox = if *is_enabled {
                if *has_templates {
                    Span::styled("[~]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("[X]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                }
            } else {
                Span::styled("[ ]", Style::default().fg(Color::DarkGray))
            };

            let row_style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(0, 150, 0))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let value_display = if *is_list_editing {
                let cursor_char = if (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() / 500) % 2 == 0 {
                    "█"
                } else {
                    " "
                };
                let before: String = list_input.chars().take(app.get_list_input_cursor()).collect();
                let after: String = list_input.chars().skip(app.get_list_input_cursor()).collect();
                format!(" {}{}{}", before, cursor_char, after)
            } else if *is_editing {
                let cursor_char = if (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() / 500) % 2 == 0 {
                    "█"
                } else {
                    " "
                };
                if let Some(val) = edit_value {
                    format!(" {}{}{}", &val[..app.settings_edit_cursor.min(val.len())], cursor_char, &val[app.settings_edit_cursor.min(val.len())..])
                } else {
                    String::new()
                }
            } else if !edit_value.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                format!(" {}", edit_value.as_ref().unwrap())
            } else if !templates.is_empty() {
                format!(" ({})", templates.join(", "))
            } else {
                String::new()
            };

            let content = Line::from(vec![
                Span::styled(format!("{}  {:<18}", checkbox, label), row_style),
                Span::styled(description.to_string(), Style::default().fg(Color::DarkGray)),
                Span::styled(value_display, Style::default().fg(Color::Cyan)),
            ]);
            ListItem::new(content)
        })
        .collect();

    let settings_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .padding(Padding::horizontal(1)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(settings_list, chunks[1]);
}

fn get_settings_for_category(app: &App) -> Vec<(&'static str, &'static str, bool, bool, bool, Option<String>, Vec<String>, bool, String)> {
    let category = app.settings_categories[app.active_settings_category];
    match category {
        SettingsCategory::General => {
            vec![
                (
                    "Recipes URL",
                    "Path to recipes directory",
                    !app.config.recipes_url.is_empty(),
                    false,
                    app.settings_edit_index.map(|(_, c)| c) == Some(0),
                    Some(app.config.recipes_url.clone()),
                    vec![],
                    false,
                    String::new(),
                ),
                (
                    "Solutions URL",
                    "Path to solutions directory",
                    !app.config.solutions_url.is_empty(),
                    false,
                    app.settings_edit_index.map(|(_, c)| c) == Some(1),
                    Some(app.config.solutions_url.clone()),
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
                    app.config.init_git.mode == ToggleMode::YesSome,
                    false,
                    None,
                    app.get_toggle_templates(SettingsCategory::Git, 0),
                    app.is_list_editing(0),
                    app.get_list_input().to_string(),
                ),
                (
                    "Create .gitignore",
                    "Create local .gitignore file",
                    app.is_toggle_enabled(SettingsCategory::Git, 1),
                    app.config.create_local_gitignore.mode == ToggleMode::YesSome,
                    false,
                    None,
                    app.get_toggle_templates(SettingsCategory::Git, 1),
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
                    app.config.open_terminal.mode == ToggleMode::YesSome,
                    false,
                    None,
                    app.get_toggle_templates(SettingsCategory::Actions, 0),
                    app.is_list_editing(0),
                    app.get_list_input().to_string(),
                ),
                (
                    "Open IDE",
                    "Open IDE after creation",
                    app.is_toggle_enabled(SettingsCategory::Actions, 1),
                    app.config.open_ide.mode == ToggleMode::YesSome,
                    false,
                    None,
                    app.get_toggle_templates(SettingsCategory::Actions, 1),
                    app.is_list_editing(1),
                    app.get_list_input().to_string(),
                ),
            ]
        }
    }
}

fn render_language_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, f.area());

    f.render_widget(Clear, area);

    let filtered = app.get_filtered_languages();

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(orig_idx, lang)| {
            let lang_color = get_language_color(lang);
            let is_selected = *orig_idx == app.selected_language_index;

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(lang_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(lang_color)
            };

            let marker = if is_selected { "> " } else { "  " };
            ListItem::new(Line::from(Span::styled(format!("{}{}", marker, lang), style)))
        })
        .collect();

    let search_text = if app.language_search.is_empty() {
        "Type to filter...".to_string()
    } else {
        app.language_search.clone()
    };

    let search_style = if app.language_search.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let search_widget = Paragraph::new(search_text)
        .style(search_style)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Search ")
                .padding(Padding::horizontal(1)),
        );

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Select Language ")
                .padding(Padding::horizontal(1)),
        );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .split(area);

    f.render_widget(search_widget, chunks[0]);
    f.render_widget(list, chunks[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
