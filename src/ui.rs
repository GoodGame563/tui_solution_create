use std::str::FromStr;

use crate::app::{App, AppState, SettingsCategory};
use crate::settings::get_settings_for_category;
use crate::structs::ConfigFileCreation;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Color,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Padding, Paragraph, Tabs},
};

pub fn ui(f: &mut Frame, app: &App) {
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
    if app.template_popup_open {
        render_template_popup(f, app);
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
                .title(Line::from(vec![Span::styled(
                    "solution_create",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )]))
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
    let status_text = if let AppState::Creating {
        status_message,
        total_progress,
        ..
    } = &app.state
    {
        let progress_pct = (*total_progress * 100.0) as u16;
        Line::from(vec![
            Span::styled(
                format!("{:3}% ", progress_pct),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(status_message, Style::default().fg(Color::Yellow)),
        ])
    } else if let AppState::Done = &app.state {
        Line::from(vec![Span::styled(
            "Project created! Press Enter to continue",
            Style::default().fg(Color::Green),
        )])
    } else if let Some((msg, _)) = &app.status_message {
        Line::from(vec![Span::raw(msg.clone())])
    } else if app.active_main_tab == 0 {
        if app.input_active {
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" Create  "),
                Span::styled("Esc", Style::default().fg(Color::Red)),
                Span::raw(" Cancel  "),
                Span::styled("Ctrl+V", Style::default().fg(Color::Cyan)),
                Span::raw(" Paste"),
            ])
        } else {
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" Edit URL  "),
                Span::styled("~", Style::default().fg(Color::Cyan)),
                Span::raw(" Language  "),
                Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
                Span::raw(" Scroll  "),
                Span::styled("Tab", Style::default().fg(Color::Magenta)),
                Span::raw(" Settings  "),
                Span::styled("q", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ])
        }
    } else {
        if app.settings_edit_index.is_some() {
            let category = app.settings_categories[app.active_settings_category];
            if category == SettingsCategory::General {
                Line::from(vec![
                    Span::styled("Enter", Style::default().fg(Color::Green)),
                    Span::raw(" Save  "),
                    Span::styled("Esc", Style::default().fg(Color::Red)),
                    Span::raw(" Cancel  "),
                    Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
                    Span::raw(" Switch  "),
                    Span::styled("←/→", Style::default().fg(Color::Magenta)),
                    Span::raw(" Edit"),
                ])
            } else {
                Line::from(vec![
                    Span::styled("Enter", Style::default().fg(Color::Green)),
                    Span::raw(" Save  "),
                    Span::styled("Esc", Style::default().fg(Color::Red)),
                    Span::raw(" Cancel"),
                ])
            }
        } else if app.template_popup_open {
            Line::from(vec![
                Span::styled("Space", Style::default().fg(Color::Green)),
                Span::raw(" Toggle  "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" Save  "),
                Span::styled("Esc", Style::default().fg(Color::Red)),
                Span::raw(" Cancel"),
            ])
        } else if app.list_edit_index.is_some() {
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" Save  "),
                Span::styled("Esc", Style::default().fg(Color::Red)),
                Span::raw(" Cancel"),
            ])
        } else {
            Line::from(vec![
                Span::styled("Enter/Space", Style::default().fg(Color::Green)),
                Span::raw(" Toggle  "),
                Span::styled("c", Style::default().fg(Color::Cyan)),
                Span::raw(" Select recipes  "),
                Span::styled("s", Style::default().fg(Color::Green)),
                Span::raw(" Save  "),
                Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
                Span::raw(" Navigate  "),
                Span::styled("←/→", Style::default().fg(Color::Magenta)),
                Span::raw(" Switch  "),
                Span::styled("Tab", Style::default().fg(Color::Magenta)),
                Span::raw(" Recipes  "),
                Span::styled("q", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ])
        }
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
                .constraints([Constraint::Length(3), Constraint::Min(5)])
                .split(area);

            let select_recipe = match app.selected_recipe.as_ref() {
                Some(s) => s,
                None => &ConfigFileCreation::default(),
            };
            let selected_lang = select_recipe.name.clone();
            let lang_color = get_color_from_name(&select_recipe.color);

            let input_display = if app.input_active {
                let cursor_char = if (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    / 500)
                    % 2
                    == 0
                {
                    "█"
                } else {
                    " "
                };
                let before: String = app.url_input.chars().take(app.cursor_position).collect();
                let after: String = app.url_input.chars().skip(app.cursor_position).collect();
                format!("{}{}{}", before, cursor_char, after)
            } else if app.url_input.is_empty() {
                "Press Enter to enter LeetCode URL".to_string()
            } else {
                app.url_input.clone()
            };

            let border_style = if app.input_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let title_style = if app.input_active {
                Style::default().fg(lang_color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let input_widget = Paragraph::new(input_display.as_str())
                .style(Style::default().fg(Color::White))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" LeetCode URL | Language: {} ", selected_lang))
                        .title_style(title_style)
                        .border_style(border_style)
                        .padding(Padding::horizontal(1)),
                );

            f.render_widget(input_widget, left_chunks[0]);

            let visible_projects = area.height.saturating_sub(6) as usize;
            let visible_projects = if visible_projects < 1 {
                1
            } else {
                visible_projects
            };
            let start = app.projects_scroll;

            let projects: Vec<ListItem> = app
                .projects
                .iter()
                .skip(start)
                .take(visible_projects)
                .map(|p| {
                    let mut lang_color = Color::White;
                    for r in &app.recipes {
                        if r.name == p.recipes_name {
                            lang_color = get_color_from_name(&r.color);
                            break;
                        }
                    }
                    let content = Line::from(vec![
                        Span::styled(
                            format!("{:<12}", p.recipes_name),
                            Style::default().fg(lang_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" > ", Style::default().fg(Color::DarkGray)),
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
                        .border_style(Style::default().fg(Color::DarkGray))
                        .padding(Padding::horizontal(1)),
                )
                .style(Style::default().fg(Color::White));

            f.render_widget(projects_list, left_chunks[1]);
        }
        AppState::Creating {
            total_progress,
            current_stage,
            status_message,
            stage_progress,
            ..
        } => {
            render_creation_screen(
                f,
                area,
                *total_progress,
                *current_stage,
                status_message,
                *stage_progress,
            );
        }
    }
}

fn get_color_from_name(select_name: &Option<String>) -> Color {
    match select_name {
        Some(s) => match Color::from_str(s) {
            Ok(c) => c,
            Err(_) => Color::White,
        },
        None => return Color::White,
    }
}

fn render_creation_screen(
    f: &mut Frame,
    area: Rect,
    total_progress: f64,
    current_stage: crate::app::CreationStage,
    _status_message: &str,
    stage_progress: f64,
) {
    use crate::app::CreationStage;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("Создание решения...")
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .padding(Padding::horizontal(1)),
        );

    f.render_widget(title, chunks[0]);

    // Отображение всех стадий с индикаторами
    let all_stages = [
        CreationStage::ExtractingSlug,
        CreationStage::FetchingProblem,
        CreationStage::CreatingProject,
        CreationStage::RunningCommands,
        CreationStage::Finalizing,
    ];

    let stages: Vec<ListItem> = all_stages
        .iter()
        .enumerate()
        .map(|(i, &stage)| {
            let is_current = stage == current_stage;
            let is_completed = all_stages.iter().position(|&s| s == current_stage) > Some(i);

            let style = if is_current {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_completed {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let prefix = if is_completed {
                "✓"
            } else if is_current {
                "⋯"
            } else {
                "○"
            };

            let stage_progress_bar = if is_current {
                // Мини-прогресс бар внутри стадии
                let width = 10;
                let filled = ((width as f64) * stage_progress.clamp(0.0, 1.0)) as usize;
                let bar: String = (0..width)
                    .map(|j| if j < filled { '█' } else { '░' })
                    .collect();
                format!(" [{}]", bar)
            } else {
                String::new()
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::raw(" "),
                Span::styled(stage.as_str(), style),
                Span::raw(stage_progress_bar),
            ]))
        })
        .collect();

    let stages_list = List::new(stages).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .title(format!(" Этап: {} ", current_stage.as_str()))
            .padding(Padding::horizontal(1)),
    );

    f.render_widget(stages_list, chunks[1]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Green))
        .percent((total_progress * 100.0) as u16)
        .label(Span::styled(
            format!("{:.1}%", total_progress * 100.0),
            Style::default().fg(Color::White),
        ));

    f.render_widget(gauge, chunks[2]);
}

fn render_settings_tab(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let category_tabs: Vec<Line> = app
        .settings_categories
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
        .map(
            |(
                i,
                (
                    label,
                    description,
                    is_enabled,
                    has_templates,
                    is_editing,
                    edit_value,
                    templates,
                    is_list_editing,
                    list_input,
                ),
            )| {
                let is_selected = i == app.settings_cursor;

                let checkbox = if *is_enabled {
                    if *has_templates {
                        Span::styled(
                            "[~]",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::styled(
                            "[X]",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
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
                    let cursor_char = if (std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                        / 500)
                        % 2
                        == 0
                    {
                        "█"
                    } else {
                        " "
                    };
                    let before: String = list_input
                        .chars()
                        .take(app.get_list_input_cursor())
                        .collect();
                    let after: String = list_input
                        .chars()
                        .skip(app.get_list_input_cursor())
                        .collect();
                    format!(" {}{}{}", before, cursor_char, after)
                } else if *is_editing {
                    let cursor_char = if (std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                        / 500)
                        % 2
                        == 0
                    {
                        "▌"
                    } else {
                        " "
                    };
                    if let Some(val) = edit_value {
                        let before: String = val.chars().take(app.settings_edit_cursor).collect();
                        let after: String = val.chars().skip(app.settings_edit_cursor).collect();
                        format!(" {}{}{}", before, cursor_char, after)
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
                    Span::styled(
                        description.to_string(),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(value_display, Style::default().fg(Color::Cyan)),
                ]);
                ListItem::new(content)
            },
        )
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

fn render_language_popup(f: &mut Frame, app: &App) {
    let in_modal = app.language_popup_in_modal;
    let area = if in_modal {
        centered_rect(60, 40, f.area())
    } else {
        centered_rect(60, 50, f.area())
    };

    f.render_widget(Clear, area);

    let filtered = app.get_filtered_languages();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, (_orig_idx, lang))| {
            let lang_color = get_color_from_name(&lang.color);
            let is_selected = i == app.language_filtered_index;

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(lang_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(lang_color)
            };

            let marker = if is_selected { "> " } else { "  " };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}", marker, lang.name),
                style,
            )))
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

    let title = if in_modal {
        " Select Language (Enter to confirm) "
    } else {
        " Select Language "
    };

    let search_widget = Paragraph::new(search_text).style(search_style).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Search ")
            .padding(Padding::horizontal(1)),
    );

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(title)
            .padding(Padding::horizontal(1)),
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(area);

    f.render_widget(search_widget, chunks[0]);
    f.render_widget(list, chunks[1]);
}

fn render_template_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, f.area());

    f.render_widget(Clear, area);

    let items: Vec<ListItem> = app
        .template_popup_recipes
        .iter()
        .enumerate()
        .map(|(i, template)| {
            let is_selected = i == app.template_popup_scroll;
            let is_checked = app.template_popup_selected.get(i).copied().unwrap_or(false);

            let checkbox = if is_checked {
                Span::styled(
                    "[X]",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("[ ]", Style::default().fg(Color::DarkGray))
            };

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(0, 150, 0))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let marker = if is_selected { "> " } else { "  " };
            ListItem::new(Line::from(vec![
                checkbox,
                Span::raw(" "),
                Span::styled(format!("{}{}", marker, template), style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Select Recipes (Space to toggle, Enter to save) ")
            .padding(Padding::horizontal(1)),
    );

    f.render_widget(list, area);
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
