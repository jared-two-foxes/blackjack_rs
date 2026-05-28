use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use uuid::Uuid;

use crate::state::{card_display, App, Screen};
use crate::types::ActionMsg;

pub fn render(f: &mut Frame, app: &App) {
    match &app.screen {
        Screen::Lobby => render_lobby(f, app),
        Screen::Betting { .. } => render_betting(f, app),
        Screen::Game { .. } => render_game(f, app),
        Screen::Result { .. } => render_result(f, app),
    }
}

pub fn render_lobby(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("♠ Blackjack — Lobby")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(title, chunks[0]);

    // Table list
    let items: Vec<ListItem> = app
        .tables
        .iter()
        .map(|t| {
            let countdown = t
                .seconds_remaining
                .map(|s| format!(" ({}s)", s))
                .unwrap_or_default();
            let text = format!(
                "  {} | {} | {} players{}",
                &t.id.to_string()[..8],
                t.state,
                t.player_count,
                countdown
            );
            ListItem::new(text)
        })
        .collect();

    let mut list_state = ListState::default();
    if !app.tables.is_empty() {
        list_state.select(Some(app.selected_table));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Tables"))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], &mut list_state);

    // Footer
    let footer = Paragraph::new("[↑↓] navigate  [Enter] join  [Q] quit")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[2]);
}

pub fn render_betting(f: &mut Frame, app: &App) {
    let area = f.area();
    let (table_id, _hand_id) = match &app.screen {
        Screen::Betting { table_id, hand_id } => (*table_id, *hand_id),
        _ => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    // Title
    let title = Paragraph::new(format!(
        "♠ Blackjack — Betting at table {}",
        &table_id.to_string()[..8]
    ))
    .block(Block::default().borders(Borders::ALL))
    .style(Style::default().fg(Color::Yellow));
    f.render_widget(title, chunks[0]);

    // Info
    let countdown = app
        .table_state
        .as_ref()
        .and_then(|ts| ts.seconds_remaining)
        .map(|s| format!("  Countdown: {}s", s))
        .unwrap_or_else(|| "  Countdown: —".to_string());
    let info_text = format!("  Balance: {}  |{}", app.balance, countdown);
    let info =
        Paragraph::new(info_text).block(Block::default().borders(Borders::ALL).title("Info"));
    f.render_widget(info, chunks[1]);

    // Bet input
    let bet_display = format!("  Bet amount: {}_", app.bet_input);
    let bet_input = Paragraph::new(bet_display)
        .block(Block::default().borders(Borders::ALL).title("Enter Bet"))
        .style(Style::default().fg(Color::White));
    f.render_widget(bet_input, chunks[2]);

    // Status message
    if let Some(msg) = &app.status_msg {
        let status = Paragraph::new(format!("  {}", msg)).style(Style::default().fg(Color::Red));
        f.render_widget(status, chunks[3]);
    }

    // Footer
    let footer = Paragraph::new("[Enter] confirm bet  [Esc] leave")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[4]);
}

pub fn render_game(f: &mut Frame, app: &App) {
    let area = f.area();
    let (table_id, hand_id) = match &app.screen {
        Screen::Game { table_id, hand_id } => (*table_id, *hand_id),
        _ => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(5),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(area);

    // Header with balance
    let header_text = format!(
        "♠ Blackjack — Table {}   Balance: {}",
        &table_id.to_string()[..8],
        app.balance
    );
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(header, chunks[0]);

    let ts = match &app.table_state {
        Some(ts) => ts,
        None => {
            let loading = Paragraph::new("  Loading table state...");
            f.render_widget(loading, chunks[1]);
            return;
        }
    };

    // Dealer panel
    let dealer_cards = dealer_cards_display(ts, table_id);
    let dealer_text = format!("  Dealer: {}", dealer_cards);
    let dealer_panel =
        Paragraph::new(dealer_text).block(Block::default().borders(Borders::ALL).title("Dealer"));
    f.render_widget(dealer_panel, chunks[1]);

    // All player hands
    let player_lines: Vec<Line> = ts
        .hands
        .iter()
        .filter(|h| h.hand.player != h.hand.dealer) // skip dealer hand
        .map(|h| {
            let cards_str: Vec<String> = h.cards.iter().map(card_display).collect();
            let state_str = h.state.as_deref().unwrap_or("active");
            let marker = if h.hand.id == hand_id { "▶ " } else { "  " };
            let pid_short = &h.hand.player.to_string()[..8];
            Line::from(format!(
                "{}Player {} | {} | {}",
                marker,
                pid_short,
                cards_str.join(" "),
                state_str
            ))
        })
        .collect();

    let hands_widget =
        Paragraph::new(player_lines).block(Block::default().borders(Borders::ALL).title("Hands"));
    f.render_widget(hands_widget, chunks[2]);

    // My hand + action bar
    let my_hand_info = ts.hands.iter().find(|h| h.hand.id == hand_id);
    let my_cards_str = my_hand_info
        .map(|h| {
            h.cards
                .iter()
                .map(card_display)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|| "—".to_string());

    let actions = crate::state::available_actions(app);
    let action_spans: Vec<Span> = actions
        .iter()
        .enumerate()
        .flat_map(|(i, a)| {
            let key = match a {
                ActionMsg::Hit => "[H]Hit",
                ActionMsg::Hold => "[S]Hold",
                ActionMsg::DoubleDown => "[D]Double",
                ActionMsg::Split => "[P]Split",
                ActionMsg::Surrender => "[X]Surrender",
            };
            let is_staged = app.action_staged.as_ref() == Some(a);
            let label = if is_staged {
                format!("★ {} queued", key)
            } else {
                key.to_string()
            };
            let style = if is_staged {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            let mut spans = vec![Span::styled(label, style)];
            if i < actions.len() - 1 {
                spans.push(Span::raw("  "));
            }
            spans
        })
        .collect();

    let my_hand_text = vec![
        Line::from(format!("  My hand: {}", my_cards_str)),
        Line::from(action_spans),
    ];
    let my_hand_widget = Paragraph::new(my_hand_text)
        .block(Block::default().borders(Borders::ALL).title("Your Hand"));
    f.render_widget(my_hand_widget, chunks[3]);

    // Footer
    let footer = Paragraph::new("[H]Hit  [S]Hold  [D]Double  [P]Split  [X]Surrender")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[4]);
}

pub fn render_result(f: &mut Frame, app: &App) {
    let area = f.area();
    let (table_id,) = match &app.screen {
        Screen::Result { table_id } => (*table_id,),
        _ => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    // Title
    let title = Paragraph::new(format!(
        "♠ Blackjack — Results (table {})",
        &table_id.to_string()[..8]
    ))
    .block(Block::default().borders(Borders::ALL))
    .style(Style::default().fg(Color::Yellow));
    f.render_widget(title, chunks[0]);

    // Outcomes
    let ts = app.table_state.as_ref();
    let player_id = app.player_id;

    let outcome_lines: Vec<Line> = ts
        .map(|ts| {
            ts.outcomes
                .iter()
                .filter(|(hand_id, _)| {
                    // Show outcomes for hands belonging to this player
                    ts.hands
                        .iter()
                        .any(|h| h.hand.id == *hand_id && h.hand.player == player_id)
                })
                .map(|(hand_id, outcome_str)| {
                    let bet_amount = ts
                        .bets
                        .iter()
                        .find(|b| b.hand_id == *hand_id)
                        .map(|b| b.amount)
                        .unwrap_or(0);
                    let display = format_outcome(outcome_str, bet_amount);
                    let style = if outcome_str.contains("Won") {
                        Style::default().fg(Color::Green)
                    } else if outcome_str.contains("Lost") || outcome_str.contains("Bust") {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Yellow)
                    };
                    Line::from(Span::styled(format!("  {}", display), style))
                })
                .collect()
        })
        .unwrap_or_default();

    let balance_line = Line::from(format!("  New balance: {}", app.balance));
    let mut all_lines = outcome_lines;
    all_lines.push(Line::from(""));
    all_lines.push(balance_line);

    let results_widget =
        Paragraph::new(all_lines).block(Block::default().borders(Borders::ALL).title("Outcomes"));
    f.render_widget(results_widget, chunks[1]);

    // Footer
    let footer = Paragraph::new("[Enter] play again  [Q] quit")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[2]);
}

fn dealer_cards_display(ts: &crate::types::TableState, table_id: Uuid) -> String {
    // Dealer hand: hand.player == hand.dealer == table_id
    let dealer_hand = ts
        .hands
        .iter()
        .find(|h| h.hand.player == table_id && h.hand.dealer == table_id);

    match dealer_hand {
        None => "—".to_string(),
        Some(h) => {
            let is_active = ts.game_state == "Active";
            let mut parts: Vec<String> = h.cards.iter().map(card_display).collect();
            // During active state, server only sends 1 card; show hole card as ??
            if is_active && parts.len() == 1 {
                parts.push("??".to_string());
            }
            parts.join(" ")
        }
    }
}

fn format_outcome(outcome_str: &str, bet_amount: u32) -> String {
    match outcome_str {
        "Won" => format!("Won +{}", bet_amount),
        "Lost" | "Bust" => format!("Lost −{}", bet_amount),
        "Push" => "Push".to_string(),
        "Surrendered" => format!("Surrendered −{}", bet_amount / 2),
        other => other.to_string(),
    }
}

fn _clamp_area(area: Rect) -> Rect {
    area
}
