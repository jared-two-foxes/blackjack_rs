mod backend;
mod data_source;
mod types;
mod utils;

pub use backend::{Message, MessagePacket, Resource, Response};
pub use data_source::DataSource;
pub use types::{Action, Card, CardValue, Deck, Hand, Outcome, Suit};

use std::sync::mpsc;
use std::thread;

/* I think this is going to be a whole new thing on a new micro thread or something.

// Task_1: Check if we need to expand table count
            // lets check if we have a minimum of 'x' valid tables.
            let table_count = open_tables(&ds.hands);
            if table_count < 8 {
                // Add 'y' new empty tables for clients to sit at.
                for _ in 0..16 {
                    let game_id = ds.add_game();
                    ds.set_deck(game_id, create_deck());
                }
            }
*/

pub fn start_backend(
    ds: &mut DataSource,
) -> thread::JoinHandle<()> {
    
    thread::spawn(move || {
        loop {
            // 
            if !ds.actions.is_empty() {
                ds.process_hit_actions();
                ds.process_hold_actions();
                ds.resolve_turn();
            }
        }
    })
}
