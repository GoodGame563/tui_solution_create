mod structs;
mod generator;

use structs::{AppConfig, ToggleMode};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Table, Row},
    Frame, Terminal,
};
use ratatui_explorer::{FileExplorer, Theme};
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, Stdout};
use std::path::{Path, PathBuf};
use std::process::Command;
use tera::{Context, Tera};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task;
use toml;


#[derive(Debug, Default)]
struct Model {
    current_screen: Screen,
    config: AppConfig,
    config_path: PathBuf,
    projects_dir: PathBuf,
    templates_dir: PathBuf,
    templates: Vec<Template>,
    recent_projects: Vec<Project>,
    current_problem: Option<Problem>,
    popup: Option<Popup>,
    quit: bool,
}

#[derive(Debug, PartialEq)]
enum Screen {
    Home(HomeState),
    NewProject(NewProjectState),
    TemplatesList(TemplatesListState),
    BrowseProjects(FileExplorer),
    Settings(SettingsState),
}

#[derive(Debug, PartialEq)]
enum Popup {
    Error(String),
    Progress(String),
}

#[derive(Debug, Default, PartialEq)]
struct HomeState {
    selected: usize,
}

#[derive(Debug, Default, PartialEq)]
struct NewProjectState {
    url_input: String,
    selected_template: usize,
    step: NewProjectStep,
}

#[derive(Debug, PartialEq)]
enum NewProjectStep {
    EnterUrl,
    SelectTemplate,
    Generating,
}

impl Default for NewProjectStep {
    fn default() -> Self {
        NewProjectStep::EnterUrl
    }
}

#[derive(Debug, Default, PartialEq)]
struct TemplatesListState {
    selected: usize,
}

#[derive(Debug, Default, PartialEq)]
struct SettingsState {
    selected_field: usize,
    editing: Option<EditingField>,
    dirty: bool,
}

#[derive(Debug, PartialEq)]
enum EditingField {
    StringField(usize, String),
    ListEdit(usize, usize, String),
}

#[derive(Debug, PartialEq)]
enum Message {
    Key(KeyEvent),
    SwitchScreen(Screen),
    FetchProblem(String),
    ProblemFetched(Result<Problem>),
    CreateProject(usize),
    ProjectCreated(Result<PathBuf>),
    DeleteTemplate(usize),
    ActivateTemplate(usize),
    EditTemplate(usize),
    NewTemplate,
    GetTemplate(String),
    TemplateUpdated,
    BrowseSelect(PathBuf),
    SettingsSave,
    SettingsReset,
    SettingsFieldCycle(usize),
    SettingsEnterEdit(usize),
    SettingsUpdateString(usize, String),
    SettingsListAdd(usize, String),
    SettingsListDelete(usize, usize),
    PopupShow(Popup),
    PopupClose,
    Quit,
}

#[derive(Debug, Clone)]
struct Template {
    name: String,
    lang: String,
    path: PathBuf,
    active: bool,
    manifest: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
struct Project {
    name: String,
    lang: String,
    path: PathBuf,
    date: String,
}

#[derive(Debug, Clone)]
struct Problem {
    title: String,
    slug: String,
    difficulty: String,
    content_html: String,
    code_snippets: HashMap<String, String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;
    let (tx, rx) = mpsc::channel(32);
    let mut model = init_model()?;
    run(&mut terminal, &mut model, rx, tx).await?;
    restore_terminal(&mut terminal)?;
    Ok(())
}

fn init_model() -> Result<Model> {
    let config_dir = dirs::config_dir().unwrap_or(PathBuf::from(".")).join("leetcode-tui");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    let config = if config_path.exists() {
        let config_str = fs::read_to_string(&config_path)?;
        toml::from_str(&config_str)?
    } else {
        let default_config = AppConfig::default();
        let config_str = toml::to_string(&default_config)?;
        fs::write(&config_path, config_str)?;
        default_config
    };
    let projects_dir = PathBuf::from(config.solutions_url.clone());
    let templates_dir = config_dir.join("templates");
    fs::create_dir_all(&templates_dir)?;
    let templates = load_templates(&templates_dir)?;
    let recent_projects = load_recent_projects(&projects_dir)?;
    Ok(Model {
        current_screen: Screen::Home(HomeState::default()),
        config,
        config_path,
        projects_dir,
        templates_dir,
        templates,
        recent_projects,
        current_problem: None,
        popup: None,
        quit: false,
    })
}

fn load_templates(dir: &Path) -> Result<Vec<Template>> {
    let mut templates = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            let manifest_path = path.join("manifest.toml");
            if manifest_path.exists() {
                let manifest_str = fs::read_to_string(manifest_path)?;
                let manifest: HashMap<String, Value> = toml::from_str(&manifest_str)?;
                let meta = manifest.get("meta").and_then(|m| m.as_table()).ok_or(anyhow::anyhow!("Invalid meta"))?;
                templates.push(Template {
                    name: meta.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                    lang: meta.get("language").and_then(|l| l.as_str()).unwrap_or("").to_string(),
                    path,
                    active: meta.get("active").and_then(|a| a.as_bool()).unwrap_or(true),
                    manifest,
                });
            }
        }
    }
    Ok(templates)
}

fn load_recent_projects(dir: &Path) -> Result<Vec<Project>> {
    let mut projects = Vec::new();
    // TODO: Real logic to load recent projects from dir
    projects
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout)).context("creating terminal failed")
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    model: &mut Model,
    mut rx: Receiver<Message>,
    tx: Sender<Message>,
) -> Result<()> {
    loop {
        terminal.draw(|f| view(model, f))?;
        if let Some(msg) = handle_events(&mut rx).await? {
            let mut chain = Some(msg);
            while let Some(m) = chain {
                chain = update(model, m, tx.clone()).await;
            }
        }
        if model.quit {
            break;
        }
    }
    Ok(())
}

async fn handle_events(rx: &mut Receiver<Message>) -> Result<Option<Message>> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            return Ok(Some(Message::Key(key)));
        }
    }
    if let Ok(msg) = rx.try_recv() {
        return Ok(Some(msg));
    }
    Ok(None)
}

async fn update(model: &mut Model, msg: Message, tx: Sender<Message>) -> Option<Message> {
    match msg {
        Message::Key(key) => handle_key(model, key, tx).await,
        Message::SwitchScreen(screen) => {
            model.current_screen = screen;
            None
        }
        Message::FetchProblem(url) => {
            let slug = parse_slug(&url);
            let tx_clone = tx.clone();
            task::spawn(async move {
                let result = fetch_leetcode_problem(&slug).await;
                tx_clone.send(Message::ProblemFetched(result)).await.ok();
            });
            Some(Message::PopupShow(Popup::Progress("Fetching...".to_string())))
        }
        Message::ProblemFetched(result) => {
            match result {
                Ok(problem) => {
                    model.current_problem = Some(problem);
                    Some(Message::PopupClose)
                }
                Err(e) => Some(Message::PopupShow(Popup::Error(e.to_string()))),
            }
        }
        Message::CreateProject(idx) => {
            let template = model.templates.get(idx).cloned();
            let problem = model.current_problem.clone();
            let projects_dir = model.projects_dir.clone();
            let config = model.config.clone();
            let tx_clone = tx.clone();
            task::spawn(async move {
                let result = create_project(template, problem, &projects_dir, &config).await;
                tx_clone.send(Message::ProjectCreated(result)).await.ok();
            });
            Some(Message::PopupShow(Popup::Progress("Generating...".to_string())))
        }
        Message::ProjectCreated(result) => {
            match result {
                Ok(path) => {
                    // TODO: Add to recent_projects
                    Some(Message::PopupShow(Popup::Error(format!("Created at {}", path.display()))))
                }
                Err(e) => Some(Message::PopupShow(Popup::Error(e.to_string()))),
            }
        }
        Message::DeleteTemplate(idx) => {
            if let Some(template) = model.templates.get(idx) {
                fs::remove_dir_all(&template.path).ok();
                model.templates.remove(idx);
            }
            None
        }
        Message::ActivateTemplate(idx) => {
            if let Some(template) = model.templates.get_mut(idx) {
                template.active = !template.active;
                let manifest_path = template.path.join("manifest.toml");
                let mut manifest_str = toml::to_string(&template.manifest).unwrap_or_default();
                // TODO: Update active in manifest
                fs::write(manifest_path, manifest_str).ok();
            }
            None
        }
        Message::EditTemplate(idx) => {
            if let Some(template) = model.templates.get(idx) {
                let editor = std::env::var("EDITOR").unwrap_or("vi".to_string());
                Command::new(editor).arg(template.path.to_str().unwrap()).status().ok();
            }
            Some(Message::TemplateUpdated)
        }
        Message::NewTemplate => {
            let new_path = model.templates_dir.join("new_template");
            fs::create_dir_all(&new_path).ok();
            let manifest_path = new_path.join("manifest.toml");
            let default_manifest = r#"[meta]
name = "New Template"
language = "rust"
active = true"#;
            fs::write(manifest_path, default_manifest).ok();
            model.templates = load_templates(&model.templates_dir).unwrap_or_default();
            None
        }
        Message::GetTemplate(url) => {
            let target = model.templates_dir.join("cloned_template");
            Command::new("git").args(["clone", &url, target.to_str().unwrap()]).status().ok();
            model.templates = load_templates(&model.templates_dir).unwrap_or_default();
            None
        }
        Message::TemplateUpdated => {
            model.templates = load_templates(&model.templates_dir).unwrap_or_default();
            None
        }
        Message::BrowseSelect(path) => {
            // TODO: Apply config open_terminal/ide
            None
        }
        Message::SettingsSave => {
            let config_str = toml::to_string(&model.config)?;
            fs::write(&model.config_path, config_str)?;
            model.popup = Some(Popup::Error("Saved".to_string()));
            None
        }
        Message::SettingsReset => {
            model.config = AppConfig::default();
            None
        }
        Message::SettingsFieldCycle(field) => {
            cycle_toggle_mode(model, field);
            None
        }
        Message::SettingsEnterEdit(field) => {
            enter_edit_mode(model, field);
            None
        }
        Message::SettingsUpdateString(field, value) => {
            update_string_field(model, field, value);
            None
        }
        Message::SettingsListAdd(field, item) => {
            add_to_list(model, field, item);
            None
        }
        Message::SettingsListDelete(field, idx) => {
            delete_from_list(model, field, idx);
            None
        }
        Message::PopupShow(popup) => {
            model.popup = Some(popup);
            None
        }
        Message::PopupClose => {
            model.popup = None;
            None
        }
        Message::Quit => {
            model.quit = true;
            None
        }
    }
}

fn parse_slug(url: &str) -> String {
    url.split('/').rev().skip(1).next().unwrap_or("").to_string()
}

async fn fetch_leetcode_problem(slug: &str) -> Result<Problem> {
    let client = reqwest::Client::new();
    let query = r#"query getQuestionDetail($titleSlug: String!) {
        question(titleSlug: $titleSlug) {
            questionId
            questionTitle
            questionTitleSlug
            content
            difficulty
            codeDefinition
        }
    }"#;
    let variables = json!({ "titleSlug": slug });
    let payload = json!({
        "operationName": "getQuestionDetail",
        "variables": variables,
        "query": query
    });
    let res = client
        .post("https://leetcode.com/graphql")
        .header(CONTENT_TYPE, "application/json")
        .json(&payload)
        .send()
        .await?;
    let json: Value = res.json().await?;
    let question = &json["data"]["question"];
    let code_definition: Vec<Value> = serde_json::from_value(question["codeDefinition"].clone())?;
    let mut code_snippets = HashMap::new();
    for def in code_definition {
        let lang = def["value"].as_str().unwrap_or("").to_string();
        let code = def["defaultCode"].as_str().unwrap_or("").to_string();
        code_snippets.insert(lang, code);
    }
    Ok(Problem {
        title: question["questionTitle"].as_str().unwrap_or("").to_string(),
        slug: question["questionTitleSlug"].as_str().unwrap_or("").to_string(),
        difficulty: question["difficulty"].as_str().unwrap_or("").to_string(),
        content_html: question["content"].as_str().unwrap_or("").to_string(),
        code_snippets,
    })
}

async fn create_project(template: Option<Template>, problem: Option<Problem>, projects_dir: &Path, config: &AppConfig) -> Result<PathBuf> {
    // TODO: Use tera to render template, generate README from content_html (use htmd), apply config init_git etc.
    let path = projects_dir.join("new_project");
    fs::create_dir_all(&path)?;
    Ok(path)
}

async fn handle_key(model: &mut Model, key: KeyEvent, tx: Sender<Message>) -> Option<Message> {
    if let Some(popup) = &model.popup {
        if key.code == KeyCode::Enter {
            return Some(Message::PopupClose);
        }
        return None;
    }
    match &mut model.current_screen {
        Screen::Home(state) => handle_home_key(model, state, key),
        Screen::NewProject(state) => handle_new_project_key(model, state, key, tx),
        Screen::TemplatesList(state) => handle_templates_list_key(model, state, key),
        Screen::BrowseProjects(explorer) => handle_browse_projects_key(model, explorer, key),
        Screen::Settings(state) => handle_settings_key(model, state, key),
    }
}

fn handle_home_key(model: &Model, state: &mut HomeState, key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Down => {
            state.selected = (state.selected + 1) % model.recent_projects.len();
            None
        }
        KeyCode::Up => {
            state.selected = state.selected.saturating_sub(1);
            None
        }
        KeyCode::Char('n') => Some(Message::SwitchScreen(Screen::NewProject(NewProjectState::default()))),
        KeyCode::Char('t') => Some(Message::SwitchScreen(Screen::TemplatesList(TemplatesListState::default()))),
        KeyCode::Char('b') => {
            let theme = Theme::default();
            let explorer = FileExplorer::new(&model.projects_dir, theme).unwrap_or_default();
            Some(Message::SwitchScreen(Screen::BrowseProjects(explorer)))
        },
        KeyCode::Char('s') => Some(Message::SwitchScreen(Screen::Settings(SettingsState::default()))),
        KeyCode::Char('q') => Some(Message::Quit),
        _ => None,
    }
}

fn handle_new_project_key(model: &Model, state: &mut NewProjectState, key: KeyEvent, tx: Sender<Message>) -> Option<Message> {
    match state.step {
        NewProjectStep::EnterUrl => {
            match key.code {
                KeyCode::Char(c) => {
                    state.url_input.push(c);
                    None
                }
                KeyCode::Backspace => {
                    state.url_input.pop();
                    None
                }
                KeyCode::Enter => {
                    state.step = NewProjectStep::SelectTemplate;
                    Some(Message::FetchProblem(state.url_input.clone()))
                }
                _ => None,
            }
        }
        NewProjectStep::SelectTemplate => {
            match key.code {
                KeyCode::Down => {
                    state.selected_template = (state.selected_template + 1) % model.templates.len();
                    None
                }
                KeyCode::Up => {
                    state.selected_template = state.selected_template.saturating_sub(1);
                    None
                }
                KeyCode::Enter => {
                    state.step = NewProjectStep::Generating;
                    Some(Message::CreateProject(state.selected_template))
                }
                _ => None,
            }
        }
        NewProjectStep::Generating => None,
    }
}

fn handle_templates_list_key(model: &Model, state: &mut TemplatesListState, key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Down => {
            state.selected = (state.selected + 1) % model.templates.len();
            None
        }
        KeyCode::Up => {
            state.selected = state.selected.saturating_sub(1);
            None
        }
        KeyCode::Char('d') => Some(Message::DeleteTemplate(state.selected)),
        KeyCode::Char('a') => Some(Message::ActivateTemplate(state.selected)),
        KeyCode::Char('e') => Some(Message::EditTemplate(state.selected)),
        KeyCode::Char('n') => Some(Message::NewTemplate),
        KeyCode::Char('g') => Some(Message::GetTemplate("https://example.com/template.git".to_string())), // Placeholder URL input
        _ => None,
    }
}

fn handle_browse_projects_key(model: &Model, explorer: &mut FileExplorer, key: KeyEvent) -> Option<Message> {
    explorer.handle_key(key);
    if key.code == KeyCode::Enter {
        if let Some(path) = explorer.current().map(|e| e.path().to_path_buf()) {
            return Some(Message::BrowseSelect(path));
        }
    }
    None
}

fn handle_settings_key(model: &mut Model, state: &mut SettingsState, key: KeyEvent) -> Option<Message> {
    match &mut state.editing {
        None => {
            match key.code {
                KeyCode::Down => {
                    state.selected_field = (state.selected_field + 1) % 6;
                    None
                }
                KeyCode::Up => {
                    state.selected_field = state.selected_field.saturating_sub(1);
                    None
                }
                KeyCode::Enter => Some(Message::SettingsEnterEdit(state.selected_field)),
                KeyCode::Char(' ') | KeyCode::Tab => Some(Message::SettingsFieldCycle(state.selected_field)),
                KeyCode::Char('s') => Some(Message::SettingsSave),
                KeyCode::Char('r') => Some(Message::SettingsReset),
                KeyCode::Esc | KeyCode::Char('q') => Some(Message::SwitchScreen(Screen::Home(HomeState::default()))),
                _ => None,
            }
        }
        Some(EditingField::StringField(idx, value)) => {
            match key.code {
                KeyCode::Char(c) => {
                    value.push(c);
                    None
                }
                KeyCode::Backspace => {
                    value.pop();
                    None
                }
                KeyCode::Enter => {
                    state.editing = None;
                    state.dirty = true;
                    Some(Message::SettingsUpdateString(*idx, value.clone()))
                }
                _ => None,
            }
        }
        Some(EditingField::ListEdit(field, item_idx, input)) => {
            match key.code {
                KeyCode::Char(c) => {
                    input.push(c);
                    None
                }
                KeyCode::Backspace => {
                    input.pop();
                    None
                }
                KeyCode::Enter => {
                    state.editing = None;
                    Some(Message::SettingsListAdd(*field, input.clone()))
                }
                KeyCode::Char('d') => Some(Message::SettingsListDelete(*field, *item_idx)),
                _ => None,
            }
        }
    }
}

fn cycle_toggle_mode(model: &mut Model, field: usize) {
    let modes = [ToggleMode::No, ToggleMode::Yes, ToggleMode::YesSome];
    let current_idx = match field {
        0 => match model.config.init_git.mode {
            ToggleMode::No => 0,
            ToggleMode::Yes => 1,
            ToggleMode::YesSome => 2,
        },
        // Similar for other fields
        _ => 0,
    };
    let next_idx = (current_idx + 1) % 3;
    match field {
        0 => model.config.init_git.mode = modes[next_idx].clone(),
        // Add for others
        _ => {},
    }
}

fn enter_edit_mode(model: &mut Model, field: usize) {
    let editing = match field {
        2 => Some(EditingField::StringField(field, model.config.recipes_url.clone())),
        3 => Some(EditingField::StringField(field, model.config.solutions_url.clone())),
        _ => Some(EditingField::ListEdit(field, 0, String::new())), // For toggle lists
    };
    if let Screen::Settings(state) = &mut model.current_screen {
        state.editing = editing;
    }
}

fn update_string_field(model: &mut Model, field: usize, value: String) {
    match field {
        2 => model.config.recipes_url = value,
        3 => model.config.solutions_url = value,
        _ => {},
    }
}

fn add_to_list(model: &mut Model, field: usize, item: String) {
    match field {
        0 => model.config.init_git.list.push(item),
        // Add for others
        _ => {},
    }
}

fn delete_from_list(model: &mut Model, field: usize, idx: usize) {
    match field {
        0 => if idx < model.config.init_git.list.len() { model.config.init_git.list.remove(idx); },
        // Add for others
        _ => {},
    }
}

fn view(model: &mut Model, frame: &mut Frame) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)])
        .split(frame.area())[0];

    match &mut model.current_screen {
        Screen::Home(state) => render_home(frame, layout, model, state),
        Screen::NewProject(state) => render_new_project(frame, layout, model, state),
        Screen::TemplatesList(state) => render_templates_list(frame, layout, model, state),
        Screen::BrowseProjects(explorer) => frame.render_widget(explorer, layout),
        Screen::Settings(state) => render_settings(frame, layout, state, &model.config),
    }

    if let Some(popup) = &model.popup {
        render_popup(frame, popup);
    }
}

fn render_home(frame: &mut Frame, area: ratatui::layout::Rect, model: &Model, state: &HomeState) {
    let items: Vec<ListItem> = model.recent_projects.iter().map(|p| ListItem::new(p.name.as_str())).collect();
    let list = List::new(items)
        .block(Block::default().title("Recent Projects").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Yellow));
    frame.render_widget(list, area);
}

fn render_new_project(frame: &mut Frame, area: ratatui::layout::Rect, model: &Model, state: &NewProjectState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let input = Paragraph::new(state.url_input.as_str())
        .block(Block::default().title("LeetCode URL").borders(Borders::ALL));
    frame.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = model.templates.iter().map(|t| ListItem::new(t.name.as_str())).collect();
    let list = List::new(items)
        .block(Block::default().title("Templates").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Yellow));
    frame.render_widget(list, chunks[1]);
}

fn render_templates_list(frame: &mut Frame, area: ratatui::layout::Rect, model: &Model, state: &TemplatesListState) {
    let rows: Vec<Row> = model.templates.iter().map(|t| Row::new(vec![t.name.as_str(), t.lang.as_str(), if t.active { "Active" } else { "Inactive" }])).collect();
    let table = Table::new(rows, vec![Constraint::Percentage(40), Constraint::Percentage(30), Constraint::Percentage(30)])
        .block(Block::default().title("Templates").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Yellow));
    frame.render_widget(table, area);
}

fn render_settings(frame: &mut Frame, area: ratatui::layout::Rect, state: &SettingsState, config: &AppConfig) {
    let init_git_mode = format!("{:?}", config.init_git.mode);
    let create_local_gitignore_mode = format!("{:?}", config.create_local_gitignore.mode);
    let open_terminal_mode = format!("{:?}", config.open_terminal.mode);
    let open_ide_mode = format!("{:?}", config.open_ide.mode);
    let rows: Vec<Row> = vec![
        Row::new(vec!["init_git", init_git_mode.as_str()]),
        Row::new(vec!["create_local_gitignore", create_local_gitignore_mode.as_str()]),
        Row::new(vec!["recipes_url", config.recipes_url.as_str()]),
        Row::new(vec!["solutions_url", config.solutions_url.as_str()]),
        Row::new(vec!["open_terminal", open_terminal_mode.as_str()]),
        Row::new(vec!["open_ide", open_ide_mode.as_str()]),
    ];
    let table = Table::new(rows, [Constraint::Percentage(50), Constraint::Percentage(50)])
        .block(Block::default().title("Settings").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Yellow));
    frame.render_widget(table, area);
}

fn render_popup(frame: &mut Frame, popup: &Popup) {
    let area = centered_rect(60, 20, frame.area());
    let text = match popup {
        Popup::Error(msg) => msg.as_str(),
        Popup::Progress(msg) => msg.as_str(),
    };
    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Popup").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// fn main() -> Result<()> {
//     let app_name = "tui_solution_create";
//     let cfg: AppConfig = confy::load(app_name, None)?;  

//     confy::store(app_name, None, cfg)?;
//     println!("{:?}",confy::get_configuration_file_path(app_name, None)?);
//     Ok(())
// }