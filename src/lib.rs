mod backend;
mod data_source;
mod types;
mod utils;

pub use backend::{Message, MessagePacket, Resource, Response};
pub use data_source::DataSource;
pub use types::{Action, Card, CardValue, Deck, Hand, Outcome, Suit};

use std::sync::mpsc;
use std::thread;

fn open_tables(_hands: &[Hand]) -> u32 {
    //@todo: group each hand by table and sum them all up, return the number of tables
    //      with a count less than 'x'
    unimplemented!()
}

pub fn start_backend<F: Fn() -> Deck + Send + 'static>(
    create_deck: F,
) -> (thread::JoinHandle<()>, mpsc::Sender<MessagePacket>) {
    // Setup the Server thread
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut ds = DataSource::default();
        loop {
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

            // Task_2: Process client messages
            // Lets process the inbound messages.
            backend::process(&rx, &mut ds);

            // Task_3: Advance simulation
            // And advance the game simulations.
            if !ds.actions.is_empty() {
                ds.process_hit_actions();
                ds.process_hold_actions();
                ds.resolve_turn();
            }
        }
    });

    (handle, tx)
}
