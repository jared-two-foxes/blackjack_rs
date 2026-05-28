mod api;
mod state;
mod types;
mod ui;

use std::io;
use std::time::Duration;

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::time::interval;

use state::{App, Screen};
use types::ActionMsg;

#[derive(Parser, Debug)]
#[command(name = "blackjack-client", about = "Blackjack TUI client")]
struct Cli {
    /// Server base URL
    #[arg(long, default_value = "http://127.0.0.1:3000")]
    server: String,
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let server = cli.server.clone();

    // Register panic hook to restore terminal
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create player
    let (player_id, balance) = match api::create_player(&server).await {
        Ok(v) => v,
        Err(e) => {
            restore_terminal();
            eprintln!("Failed to connect to server: {}", e);
            return Ok(());
        }
    };

    let mut app = App::new(player_id, balance);

    // Initial table fetch
    if let Ok(tables) = api::get_tables(&server).await {
        app.tables = tables;
    }

    let mut tick = interval(Duration::from_millis(500));

    let result = run_event_loop(&mut terminal, &mut app, &server, &mut tick).await;

    restore_terminal();
    result
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    server: &str,
    tick: &mut tokio::time::Interval,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        tokio::select! {
            _ = tick.tick() => {
                handle_tick(app, server).await;
            }
            _ = wait_for_key_event() => {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.kind == KeyEventKind::Press
                        && handle_input(app, key.code, server).await
                    {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn wait_for_key_event() {
    loop {
        if event::poll(Duration::from_millis(0)).unwrap_or(false) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn handle_tick(app: &mut App, server: &str) {
    match &app.screen.clone() {
        Screen::Lobby => {
            if let Ok(tables) = api::get_tables(server).await {
                app.tables = tables;
            }
        }
        Screen::Betting { table_id, hand_id } => {
            let (tid, hid) = (*table_id, *hand_id);
            if let Ok(ts) = api::get_table(server, tid).await {
                // Auto-transition: if game moved to Active, go to Game screen
                if ts.game_state == "Active" {
                    app.table_state = Some(ts);
                    app.screen = Screen::Game {
                        table_id: tid,
                        hand_id: hid,
                    };
                } else {
                    app.table_state = Some(ts);
                }
            }
        }
        Screen::Game { table_id, hand_id } => {
            let (tid, hid) = (*table_id, *hand_id);
            if let Ok(ts) = api::get_table(server, tid).await {
                // Auto-transition: if game moved to Result/Waiting, go to Result screen
                if ts.game_state == "Result" || ts.game_state == "Waiting" {
                    // Refresh balance
                    if let Ok((_, new_balance)) = api::get_player(server, app.player_id).await {
                        app.balance = new_balance;
                    }
                    app.table_state = Some(ts);
                    app.screen = Screen::Result { table_id: tid };
                } else {
                    app.table_state = Some(ts);
                    // Clear staged action if hand is no longer active
                    if let Some(staged) = &app.action_staged.clone() {
                        let actions = state::available_actions(app);
                        if !actions.contains(staged) {
                            app.action_staged = None;
                        }
                    }
                    let _ = hid; // suppress unused warning
                }
            }
        }
        Screen::Result { table_id } => {
            let tid = *table_id;
            if let Ok(ts) = api::get_table(server, tid).await {
                app.table_state = Some(ts);
            }
        }
    }
}

/// Returns true if the app should quit.
async fn handle_input(app: &mut App, key: KeyCode, server: &str) -> bool {
    match &app.screen.clone() {
        Screen::Lobby => match key {
            KeyCode::Up => {
                if app.selected_table > 0 {
                    app.selected_table -= 1;
                }
            }
            KeyCode::Down => {
                if app.selected_table + 1 < app.tables.len() {
                    app.selected_table += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(table) = app.tables.get(app.selected_table) {
                    let table_id = table.id;
                    match api::join_table(server, table_id, app.player_id).await {
                        Ok(resp) if !resp.rejected => {
                            let hand_id = resp.hand_id.unwrap_or_else(uuid::Uuid::new_v4);
                            app.bet_input.clear();
                            app.status_msg = None;
                            app.screen = Screen::Betting { table_id, hand_id };
                        }
                        Ok(resp) => {
                            app.status_msg = Some(
                                resp.reason
                                    .unwrap_or_else(|| "Could not join table".to_string()),
                            );
                        }
                        Err(e) => {
                            app.status_msg = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => return true,
            _ => {}
        },
        Screen::Betting { table_id, hand_id } => {
            let (tid, hid) = (*table_id, *hand_id);
            match key {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    app.bet_input.push(c);
                }
                KeyCode::Backspace => {
                    app.bet_input.pop();
                }
                KeyCode::Enter => match app.bet_input.parse::<u32>() {
                    Ok(amount) if amount > 0 => {
                        match api::place_bet(server, tid, app.player_id, amount).await {
                            Ok(()) => {
                                app.status_msg = Some(format!("Bet of {} placed!", amount));
                            }
                            Err(e) => {
                                app.status_msg = Some(format!("Bet error: {}", e));
                            }
                        }
                    }
                    _ => {
                        app.status_msg = Some("Enter a valid bet amount".to_string());
                    }
                },
                KeyCode::Esc => {
                    let _ = api::leave_table(server, tid, app.player_id).await;
                    app.table_state = None;
                    app.screen = Screen::Lobby;
                    let _ = hid;
                }
                _ => {}
            }
        }
        Screen::Game { table_id, hand_id } => {
            let (tid, hid) = (*table_id, *hand_id);
            let action = match key {
                KeyCode::Char('h') | KeyCode::Char('H') => Some(ActionMsg::Hit),
                KeyCode::Char('s') | KeyCode::Char('S') => Some(ActionMsg::Hold),
                KeyCode::Char('d') | KeyCode::Char('D') => Some(ActionMsg::DoubleDown),
                KeyCode::Char('p') | KeyCode::Char('P') => Some(ActionMsg::Split),
                KeyCode::Char('x') | KeyCode::Char('X') => Some(ActionMsg::Surrender),
                _ => None,
            };
            if let Some(action) = action {
                let available = state::available_actions(app);
                if available.contains(&action) {
                    match api::submit_action(server, hid, action.clone()).await {
                        Ok(()) => {
                            app.action_staged = Some(action);
                        }
                        Err(e) => {
                            app.status_msg = Some(format!("Action error: {}", e));
                        }
                    }
                }
            }
            let _ = tid;
        }
        Screen::Result { table_id } => {
            let tid = *table_id;
            match key {
                KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    let _ = api::leave_table(server, tid, app.player_id).await;
                    app.table_state = None;
                    app.action_staged = None;
                    app.screen = Screen::Lobby;
                    if key == KeyCode::Char('q') || key == KeyCode::Char('Q') {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }
    false
}
