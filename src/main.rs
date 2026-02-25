mod structs;
mod generator;
use structs::{AppConfig, ToggleMode, ToggleWithList};

// fn main() -> Result<()> {
//     let app_name = "tui_solution_create";
//     let cfg: AppConfig = confy::load(app_name, None)?;  

//     confy::store(app_name, None, cfg)?;
//     println!("{:?}",confy::get_configuration_file_path(app_name, None)?);
//     Ok(())
// }

use std::{io, time::Duration};

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
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap, Clear, Gauge},
    Frame, Terminal,
};


#[derive(Clone, Copy, PartialEq)]
enum Language {
    Go,
    Rust,
    Python,
}

impl Language {
    fn as_str(&self) -> &'static str {
        match self {
            Language::Go => "Go",
            Language::Rust => "Rust",
            Language::Python => "Python",
        }
    }

    fn color(&self) -> Color {
        match self {
            Language::Go => Color::Cyan,
            Language::Rust => Color::Rgb(255, 100, 100),
            Language::Python => Color::Rgb(100, 100, 255),
        }
    }
}

struct Project {
    lang: Language,
    name: String,
}

enum AppState {
    Input,
    Creating { progress: f64, stage: usize },
    Done,
}

struct App {
    tabs: Vec<&'static str>,
    active_tab_index: usize,
    url_input: String,
    cursor_position: usize,
    input_mode: bool,
    languages: Vec<Language>,
    selected_language_index: usize,
    language_list_open: bool,
    projects: Vec<Project>,
    projects_scroll: usize,
    config: AppConfig,
    settings_cursor: usize,
    state: AppState,
    creation_stages: Vec<&'static str>,
}

impl App {
    fn new() -> Self {
        Self {
            tabs: vec!["Recipes", "Settings"],
            active_tab_index: 0,
            url_input: String::new(),
            cursor_position: 0,
            input_mode: false,
            languages: vec![Language::Go, Language::Rust, Language::Python],
            selected_language_index: 1,
            language_list_open: false,
            projects: vec![
                Project {
                    lang: Language::Rust,
                    name: "tui-app".to_string(),
                },
                Project {
                    lang: Language::Python,
                    name: "data-scraper".to_string(),
                },
                Project {
                    lang: Language::Go,
                    name: "web-server".to_string(),
                },
                Project {
                    lang: Language::Rust,
                    name: "cli-tool".to_string(),
                },
                Project {
                    lang: Language::Python,
                    name: "automation-bot".to_string(),
                },
                Project {
                    lang: Language::Go,
                    name: "api-gateway".to_string(),
                },
                Project {
                    lang: Language::Rust,
                    name: "blockchain-node".to_string(),
                },
            ],
            projects_scroll: 0,
            config: AppConfig::default(),
            settings_cursor: 0,
            state: AppState::Input,
            creation_stages: vec![
                "Initializing project...",
                "Cloning repository...",
                "Installing dependencies...",
                "Configuring environment...",
                "Building solution...",
                "Finalizing...",
            ],
        }
    }

    fn next_tab(&mut self) {
        self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
    }

    fn previous_tab(&mut self) {
        if self.active_tab_index > 0 {
            self.active_tab_index -= 1;
        } else {
            self.active_tab_index = self.tabs.len() - 1;
        }
    }

    fn enter_input_mode(&mut self) {
        self.input_mode = true;
    }

    fn leave_input_mode(&mut self) {
        self.input_mode = false;
        self.language_list_open = false;
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

    fn toggle_language_list(&mut self) {
        self.language_list_open = !self.language_list_open;
    }

    fn select_language(&mut self, index: usize) {
        if index < self.languages.len() {
            self.selected_language_index = index;
            self.language_list_open = false;
        }
    }

    fn move_language_selection(&mut self, direction: i8) {
        if direction > 0 {
            if self.selected_language_index < self.languages.len() - 1 {
                self.selected_language_index += 1;
            }
        } else {
            if self.selected_language_index > 0 {
                self.selected_language_index -= 1;
            }
        }
    }

    fn selected_language(&self) -> Language {
        self.languages[self.selected_language_index]
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

    fn move_settings_cursor(&mut self, direction: i8) {
        let settings_count = 6;
        if direction > 0 {
            if self.settings_cursor < settings_count - 1 {
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

    fn toggle_setting(&mut self) {
        self.deactivate_all_list_inputs();
        
        match self.settings_cursor {
            0 => {
                self.config.init_git.mode = match self.config.init_git.mode {
                    ToggleMode::No => ToggleMode::Yes,
                    ToggleMode::Yes => ToggleMode::YesSome,
                    ToggleMode::YesSome => ToggleMode::No,
                };
                if let ToggleMode::YesSome = self.config.init_git.mode {
                    self.config.init_git.list_input_active = true;
                }
            }
            1 => {
                self.config.create_local_gitignore.mode = match self.config.create_local_gitignore.mode {
                    ToggleMode::No => ToggleMode::Yes,
                    ToggleMode::Yes => ToggleMode::YesSome,
                    ToggleMode::YesSome => ToggleMode::No,
                };
                if let ToggleMode::YesSome = self.config.create_local_gitignore.mode {
                    self.config.create_local_gitignore.list_input_active = true;
                }
            }
            2 => {}
            3 => {}
            4 => {
                self.config.open_terminal.mode = match self.config.open_terminal.mode {
                    ToggleMode::No => ToggleMode::Yes,
                    ToggleMode::Yes => ToggleMode::YesSome,
                    ToggleMode::YesSome => ToggleMode::No,
                };
                if let ToggleMode::YesSome = self.config.open_terminal.mode {
                    self.config.open_terminal.list_input_active = true;
                }
            }
            5 => {
                self.config.open_ide.mode = match self.config.open_ide.mode {
                    ToggleMode::No => ToggleMode::Yes,
                    ToggleMode::Yes => ToggleMode::YesSome,
                    ToggleMode::YesSome => ToggleMode::No,
                };
                if let ToggleMode::YesSome = self.config.open_ide.mode {
                    self.config.open_ide.list_input_active = true;
                }
            }
            _ => {}
        }
    }

    fn get_toggle_text(&self, toggle: &ToggleWithList) -> String {
        match toggle.mode {
            ToggleMode::No => "No".to_string(),
            ToggleMode::Yes => "Yes".to_string(),
            ToggleMode::YesSome => format!("Some ({})", toggle.list.join(", ")),
        }
    }

    fn submit_url(&mut self) {
        if !self.url_input.is_empty() {
            self.state = AppState::Creating { progress: 0.0, stage: 0 };
            self.input_mode = false;
            self.language_list_open = false;
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
                    self.projects.insert(0, Project {
                        lang: self.selected_language(),
                        name: self.url_input.clone(),
                    });
                    self.url_input.clear();
                    self.cursor_position = 0;
                }
            }
        }
    }

    fn reset_creation(&mut self) {
        self.state = AppState::Input;
    }

    fn get_current_toggle(&mut self) -> Option<&mut ToggleWithList> {
        match self.settings_cursor {
            0 => Some(&mut self.config.init_git),
            1 => Some(&mut self.config.create_local_gitignore),
            4 => Some(&mut self.config.open_terminal),
            5 => Some(&mut self.config.open_ide),
            _ => None,
        }
    }

    fn update_list_input(&mut self, c: char) {
        if let Some(toggle) = self.get_current_toggle() {
            if toggle.list_input_active {
                toggle.list_input.push(c);
            }
        }
    }

    fn delete_list_input_char(&mut self) {
        if let Some(toggle) = self.get_current_toggle() {
            if toggle.list_input_active && !toggle.list_input.is_empty() {
                toggle.list_input.pop();
            }
        }
    }

    fn save_list_input(&mut self) {
        if let Some(toggle) = self.get_current_toggle() {
            if toggle.list_input_active {
                toggle.list = toggle.list_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                toggle.list_input_active = false;
                toggle.list_input.clear();
            }
        }
    }

    fn cancel_list_input(&mut self) {
        if let Some(toggle) = self.get_current_toggle() {
            toggle.list_input_active = false;
            toggle.list_input.clear();
        }
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
                                    if !app.input_mode && !app.language_list_open && app.active_tab_index == 0 {
                                        app.next_tab();
                                    } else if app.active_tab_index == 1 {
                                        app.previous_tab();
                                    }
                                }
                                KeyCode::BackTab => {
                                    if app.active_tab_index == 1 {
                                        app.previous_tab();
                                    }
                                }
                                KeyCode::Left => {
                                    if app.input_mode && !app.language_list_open && app.active_tab_index == 0 {
                                        app.move_cursor_left();
                                    } else if app.language_list_open {
                                        app.move_language_selection(-1);
                                    } else if app.active_tab_index == 0 {
                                        app.previous_tab();
                                    } else if app.active_tab_index == 1 {
                                        if let Some(toggle) = app.get_current_toggle() {
                                            if !toggle.list_input_active {
                                                app.move_settings_cursor(-1);
                                            }
                                        } else {
                                            app.move_settings_cursor(-1);
                                        }
                                    }
                                }
                                KeyCode::Right => {
                                    if app.input_mode && !app.language_list_open && app.active_tab_index == 0 {
                                        app.move_cursor_right();
                                    } else if app.language_list_open {
                                        app.move_language_selection(1);
                                    } else if app.active_tab_index == 0 {
                                        app.next_tab();
                                    } else if app.active_tab_index == 1 {
                                        if let Some(toggle) = app.get_current_toggle() {
                                            if !toggle.list_input_active {
                                                app.move_settings_cursor(1);
                                            }
                                        } else {
                                            app.move_settings_cursor(1);
                                        }
                                    }
                                }
                                KeyCode::Up => {
                                    if app.language_list_open {
                                        app.move_language_selection(-1);
                                    } else if app.active_tab_index == 1 {
                                        app.move_settings_cursor(-1);
                                    } else if app.active_tab_index == 0 {
                                        app.scroll_projects(-1);
                                    }
                                }
                                KeyCode::Down => {
                                    if app.language_list_open {
                                        app.move_language_selection(1);
                                    } else if app.active_tab_index == 1 {
                                        app.move_settings_cursor(1);
                                    } else if app.active_tab_index == 0 {
                                        app.scroll_projects(1);
                                    }
                                }
                                KeyCode::Enter => {
                                    if app.language_list_open {
                                        app.select_language(app.selected_language_index);
                                    } else if !app.input_mode && app.active_tab_index == 0 {
                                        app.enter_input_mode();
                                    } else if app.input_mode && app.active_tab_index == 0 {
                                        app.submit_url();
                                    } else if app.active_tab_index == 1 {
                                        app.toggle_setting();
                                    }
                                }
                                KeyCode::Esc => {
                                    if app.language_list_open {
                                        app.language_list_open = false;
                                    } else if app.input_mode {
                                        app.leave_input_mode();
                                    } else if app.active_tab_index == 1 {
                                        app.cancel_list_input();
                                    }
                                }
                                KeyCode::Backspace => {
                                    if app.input_mode && !app.language_list_open && app.active_tab_index == 0 {
                                        app.delete_char();
                                    } else if app.active_tab_index == 1 {
                                        app.delete_list_input_char();
                                    }
                                }
                                KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    if app.input_mode && !app.language_list_open && app.active_tab_index == 0 {
                                        app.paste_from_clipboard();
                                    }
                                }
                                KeyCode::Char('l') => {
                                    if app.input_mode && app.active_tab_index == 0 {
                                        app.toggle_language_list();
                                    }
                                }
                                KeyCode::Char(' ') => {
                                    if app.active_tab_index == 1 {
                                        app.toggle_setting();
                                    }
                                }
                                KeyCode::Char(c) => {
                                    if app.input_mode && !app.language_list_open && app.active_tab_index == 0 {
                                        app.insert_char(c);
                                    } else if app.active_tab_index == 1 {
                                        app.update_list_input(c);
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
            last_tick = std::time::Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(size);

    let titles: Vec<Line> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if app.active_tab_index == i {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            Line::from(Span::styled(*t, style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" solution_create ")
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(app.active_tab_index)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, main_chunks[0]);

    if app.active_tab_index == 0 {
        render_recipes_tab(f, app, main_chunks[1]);
    } else {
        render_settings_tab(f, app, main_chunks[1]);
    }

    let footer_text = if let AppState::Creating { stage, .. } = &app.state {
        Line::from(vec![
            Span::styled(app.creation_stages[*stage], Style::default().fg(Color::Yellow)),
        ])
    } else if let AppState::Done = &app.state {
        Line::from(vec![
            Span::styled("✓ Project created! Press Enter to continue", Style::default().fg(Color::Green)),
        ])
    } else if app.active_tab_index == 0 {
        Line::from(vec![
            Span::styled("[Ctrl+V]", Style::default().fg(Color::Yellow)),
            Span::raw(" Paste "),
            Span::styled("[l]", Style::default().fg(Color::Yellow)),
            Span::raw(" Lang "),
            Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
            Span::raw(" Scroll "),
            Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
            Span::raw(" Create "),
            Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
            Span::raw(" Settings "),
            Span::styled("[q]", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit"),
        ])
    } else {
        Line::from(vec![
            Span::styled("[Enter/Space]", Style::default().fg(Color::Yellow)),
            Span::raw(" Toggle "),
            Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate "),
            Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
            Span::raw(" Recipes "),
            Span::styled("[q]", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit"),
        ])
    };

    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);

    f.render_widget(footer, main_chunks[2]);
}

fn render_recipes_tab(f: &mut Frame, app: &App, area: Rect) {
    match &app.state {
        AppState::Input | AppState::Done => {
            let content_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(5),
                ])
                .split(area);

            let input_widget = Paragraph::new(app.url_input.as_str())
                .style(Style::default().fg(if app.input_mode { Color::Yellow } else { Color::White }))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" URL Input ")
                        .border_style(Style::default().fg(Color::White)),
                )
                .wrap(Wrap { trim: false });

            f.render_widget(input_widget, content_chunks[0]);

            if app.input_mode && !app.language_list_open {
                f.set_cursor_position((
                    content_chunks[0].x + app.cursor_position as u16 + 1,
                    content_chunks[0].y + 1,
                ));
            }

            let selected_lang = app.selected_language();

            if app.language_list_open {
                let lang_items: Vec<ListItem> = app
                    .languages
                    .iter()
                    .enumerate()
                    .map(|(i, lang)| {
                        let style = if i == app.selected_language_index {
                            Style::default()
                                .fg(Color::Black)
                                .bg(lang.color())
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(lang.color())
                        };
                        ListItem::new(Line::from(Span::styled(lang.as_str(), style)))
                    })
                    .collect();

                let lang_list = List::new(lang_items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Select Language (Enter to confirm, Esc to cancel) ")
                            .border_style(Style::default().fg(Color::Yellow)),
                    );

                let lang_area = Rect {
                    x: content_chunks[1].x,
                    y: content_chunks[1].y,
                    width: content_chunks[1].width,
                    height: content_chunks[1].height,
                };

                f.render_widget(Clear, lang_area);
                f.render_widget(lang_list, lang_area);
            } else {
                let lang_widget = Paragraph::new(selected_lang.as_str())
                    .style(Style::default().fg(selected_lang.color()).add_modifier(Modifier::BOLD))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Language (l to change) ")
                            .border_style(Style::default().fg(Color::White)),
                    )
                    .alignment(Alignment::Center);

                f.render_widget(lang_widget, content_chunks[1]);
            }

            let visible_projects = 5;
            let start = app.projects_scroll;
            let end = (start + visible_projects).min(app.projects.len());

            let projects: Vec<ListItem> = app
                .projects
                .iter()
                .skip(start)
                .take(visible_projects)
                .map(|p| {
                    let content = Line::from(vec![
                        Span::styled(
                            format!("{:<8}", p.lang.as_str()),
                            Style::default().fg(p.lang.color()).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("▸", Style::default().fg(Color::DarkGray)),
                        Span::raw(" "),
                        Span::raw(&p.name),
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
                        .border_style(Style::default().fg(Color::White)),
                )
                .style(Style::default().fg(Color::White));

            f.render_widget(projects_list, content_chunks[2]);
        }
        AppState::Creating { progress, stage } => {
            render_creation_screen(f, app, area, *progress, *stage);
        }
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
                .border_style(Style::default().fg(Color::Green)),
        );

    f.render_widget(title, chunks[0]);

    let stage_text = app.creation_stages[stage];
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
            let prefix = if i < stage { "✓ " } else if i == stage { "► " } else { "  " };
            ListItem::new(Line::from(Span::styled(format!("{}{}", prefix, s), style)))
        })
        .collect();

    let stages_list = List::new(stages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        );

    f.render_widget(stages_list, chunks[1]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Green))
        .percent((progress * 100.0) as u16)
        .label(Span::styled(format!("{:.0}%", progress * 100.0), Style::default().fg(Color::White)));

    f.render_widget(gauge, chunks[2]);
}

fn render_settings_tab(f: &mut Frame, app: &App, area: Rect) {
    let empty_string = String::new();
    let settings_items = vec![
        ("Init Git", app.get_toggle_text(&app.config.init_git), app.config.init_git.list_input_active, &app.config.init_git.list_input),
        ("Create Local .gitignore", app.get_toggle_text(&app.config.create_local_gitignore), app.config.create_local_gitignore.list_input_active, &app.config.create_local_gitignore.list_input),
        ("Recipes URL", app.config.recipes_url.clone(), false, &empty_string),
        ("Solutions URL", app.config.solutions_url.clone(), false, &empty_string),
        ("Open Terminal", app.get_toggle_text(&app.config.open_terminal), app.config.open_terminal.list_input_active, &app.config.open_terminal.list_input),
        ("Open IDE", app.get_toggle_text(&app.config.open_ide), app.config.open_ide.list_input_active, &app.config.open_ide.list_input),
    ];

    let items: Vec<ListItem> = settings_items
        .iter()
        .enumerate()
        .map(|(i, (label, value, is_input_active, input_text))| {
            let is_selected = i == app.settings_cursor;
            
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let value_style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };

            let display_value = if *is_input_active {
                format!("► [{}]_ ", input_text)
            } else {
                format!("► {}", value)
            };

            let content = Line::from(vec![
                Span::styled(format!("{:<25}", label), style),
                Span::styled(display_value, value_style),
            ]);
            ListItem::new(content)
        })
        .collect();

    let settings_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Configuration (Enter to toggle, type when Some is selected) ")
                .border_style(Style::default().fg(Color::White)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(settings_list, area);
}