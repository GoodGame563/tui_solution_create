mod structs;
mod generator;
use structs::{AppConfig, ToggleMode};

use std::{io, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
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
            Language::Rust => Color::Red,
            Language::Python => Color::Blue,
        }
    }
}

struct Project {
    lang: Language,
    name: String,
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
    loop {
        terminal.draw(|f| ui(f, app)).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Tab => {
                            if !app.input_mode && !app.language_list_open {
                                app.next_tab();
                            }
                        }
                        KeyCode::Left => {
                            if app.input_mode && !app.language_list_open {
                                app.move_cursor_left();
                            } else if app.language_list_open {
                                app.move_language_selection(-1);
                            } else {
                                app.previous_tab();
                            }
                        }
                        KeyCode::Right => {
                            if app.input_mode && !app.language_list_open {
                                app.move_cursor_right();
                            } else if app.language_list_open {
                                app.move_language_selection(1);
                            } else {
                                app.next_tab();
                            }
                        }
                        KeyCode::Up => {
                            if app.language_list_open {
                                app.move_language_selection(-1);
                            }
                        }
                        KeyCode::Down => {
                            if app.language_list_open {
                                app.move_language_selection(1);
                            }
                        }
                        KeyCode::Enter => {
                            if app.language_list_open {
                                app.select_language(app.selected_language_index);
                            } else if !app.input_mode {
                                app.enter_input_mode();
                            }
                        }
                        KeyCode::Esc => app.leave_input_mode(),
                        KeyCode::Backspace => {
                            if app.input_mode && !app.language_list_open {
                                app.delete_char();
                            }
                        }
                        KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if app.input_mode && !app.language_list_open {
                                app.paste_from_clipboard();
                            }
                        }
                        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if app.input_mode {
                                app.toggle_language_list();
                            }
                        }
                        KeyCode::Char(c) => {
                            if app.input_mode && !app.language_list_open {
                                app.insert_char(c);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(0),
        ])
        .split(size);

    let titles: Vec<Line> = app
        .tabs
        .iter()
        .map(|t| Line::from(Span::raw(*t)))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Main"))
        .select(app.active_tab_index)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, main_layout[0]);

    let input_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(main_layout[1]);

    let input_widget = Paragraph::new(app.url_input.as_str())
        .style(Style::default().fg(if app.input_mode { Color::Yellow } else { Color::White }))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Enter URL (Ctrl+V to paste)"),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(input_widget, input_layout[0]);

    if app.input_mode && !app.language_list_open {
        f.set_cursor_position((
            input_layout[0].x + app.cursor_position as u16 + 1,
            input_layout[0].y + 1,
        ));
    }

    let selected_lang = app.selected_language();
    let lang_block = if app.language_list_open {
        Block::default()
            .borders(Borders::ALL)
            .title("Select Language (Enter to confirm, Esc to cancel)")
            .style(Style::default().fg(Color::Yellow))
    } else {
        Block::default()
            .borders(Borders::ALL)
            .title("Language (Ctrl+L to change)")
    };

    if app.language_list_open {
        let lang_items: Vec<ListItem> = app
            .languages
            .iter()
            .enumerate()
            .map(|(i, lang)| {
                let style = if i == app.selected_language_index {
                    Style::default()
                        .fg(lang.color())
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(lang.color())
                };
                ListItem::new(Line::from(Span::styled(lang.as_str(), style)))
            })
            .collect();

        let lang_list = List::new(lang_items)
            .block(lang_block)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
            );

        f.render_widget(lang_list, input_layout[1]);
    } else {
        let lang_widget = Paragraph::new(selected_lang.as_str())
            .style(Style::default().fg(selected_lang.color()).add_modifier(Modifier::BOLD))
            .block(lang_block);

        f.render_widget(lang_widget, input_layout[1]);
    }

    let projects: Vec<ListItem> = app
        .projects
        .iter()
        .map(|p| {
            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", p.lang.as_str()),
                    Style::default().fg(p.lang.color()).add_modifier(Modifier::BOLD),
                ),
                Span::raw(&p.name),
            ]);
            ListItem::new(content)
        })
        .collect();

    let projects_list = List::new(projects)
        .block(Block::default().borders(Borders::ALL).title("Last Projects"));

    f.render_widget(projects_list, main_layout[2]);
}

// fn main() -> Result<()> {
//     let app_name = "tui_solution_create";
//     let cfg: AppConfig = confy::load(app_name, None)?;  

//     confy::store(app_name, None, cfg)?;
//     println!("{:?}",confy::get_configuration_file_path(app_name, None)?);
//     Ok(())
// }