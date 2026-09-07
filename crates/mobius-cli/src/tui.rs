use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use mobius_core::session::{Session, SessionFile};

use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
    
use std::{io, path::PathBuf};

pub fn run_tui(mut sessions: Vec<SessionFile>) -> Result<Option<PathBuf>, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // Enable mouse capture for scroll wheel support
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, &mut sessions);

    // Teardown cleanly regardless of error state
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    sessions: &mut Vec<SessionFile>,
) -> Result<Option<PathBuf>, io::Error> {
    let mut list_state = ListState::default();
    if !sessions.is_empty() {
        list_state.select(Some(0));
    }

    let mut scroll_offset: u16 = 0;

    loop {
        terminal.draw(|f| {
            // Split layout 35% / 65% Yazi style
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(35), Constraint::Percentage(65)].as_ref())
                .split(f.area());

            // --- Left Pane (List) ---
            let items: Vec<ListItem> = sessions
                .iter()
                .map(|s| {
                    let prefix = if s.is_active { "●" } else { "○" };
                    let name = s.filename.replace(".json", "");
                    let content = format!("{} {} | {}", prefix, name, s.preview);
                    let style = if s.is_active {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(Span::styled(content, style)))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Local Sessions (Press 'd' to delete) "),
                )
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .bg(Color::DarkGray),
                )
                .highlight_symbol("▶ ");

            f.render_stateful_widget(list, chunks[0], &mut list_state);

            // --- Right Pane (Preview) ---
            let mut preview_text = vec![];
            if let Some(i) = list_state.selected() {
                if let Some(s) = sessions.get(i) {
                    if let Ok(data) = std::fs::read_to_string(&s.path) {
                        if let Ok(session_data) = serde_json::from_str::<Session>(&data) {
                            for msg in session_data.messages {
                                if msg.role == "user" {
                                    preview_text.push(Line::from(vec![
                                        Span::styled(
                                            "User: ",
                                            Style::default()
                                                .fg(Color::Cyan)
                                                .add_modifier(Modifier::BOLD),
                                        ),
                                        Span::raw(msg.content),
                                    ]));
                                } else if msg.role == "assistant" {
                                    preview_text.push(Line::from(vec![
                                        Span::styled(
                                            "Mobius: ",
                                            Style::default()
                                                .fg(Color::Green)
                                                .add_modifier(Modifier::BOLD),
                                        ),
                                        Span::raw(msg.content),
                                    ]));
                                }
                                preview_text.push(Line::from("")); // Spacer line
                            }
                        }
                    }
                }
            }

            let preview = Paragraph::new(preview_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Conversation Preview "),
                )
                .wrap(Wrap { trim: false })
                .scroll((scroll_offset, 0));

            f.render_widget(preview, chunks[1]);
        })?;

        // Intercept inputs (Keyboard + Mouse)
        let event = event::read()?;
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = match list_state.selected() {
                        Some(i) => {
                            if i >= sessions.len().saturating_sub(1) {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    list_state.select(Some(i));
                    scroll_offset = 0; // Reset scroll when switching chats
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = match list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                sessions.len().saturating_sub(1)
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    list_state.select(Some(i));
                    scroll_offset = 0;
                }
                KeyCode::PageDown | KeyCode::Tab => {
                    scroll_offset = scroll_offset.saturating_add(5);
                }
                KeyCode::PageUp | KeyCode::BackTab => {
                    scroll_offset = scroll_offset.saturating_sub(5);
                }
                KeyCode::Enter => {
                    if let Some(i) = list_state.selected() {
                        if let Some(s) = sessions.get(i) {
                            return Ok(Some(s.path.clone()));
                        }
                    }
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    if let Some(i) = list_state.selected() {
                        if let Some(s) = sessions.get(i) {
                            if !s.is_active {
                                let _ = std::fs::remove_file(&s.path);
                                sessions.remove(i);
                                if i >= sessions.len() && !sessions.is_empty() {
                                    list_state.select(Some(sessions.len() - 1));
                                } else if sessions.is_empty() {
                                    list_state.select(None);
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            Event::Mouse(mouse_event) => match mouse_event.kind {
                MouseEventKind::ScrollDown => scroll_offset = scroll_offset.saturating_add(3),
                MouseEventKind::ScrollUp => scroll_offset = scroll_offset.saturating_sub(3),
                _ => {}
            },
            _ => {}
        }
    }
}
