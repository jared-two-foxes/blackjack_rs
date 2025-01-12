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
    //@todo: group each hand by table and sum them all up, return the 
    // number of tables with a count less than 'x'
    unimplemented!()
}



pub enum BackendStatus {
    UpdateComplete,
}

/*
@note:
This return type isnt really a send/recv pair.  the recv isnt receiving the 
sent message, its recving an engine message from the backend thread that some 
process has completed.  Also its not receiving an update based upon a response 
from the sent message, ie ita not an ack of message sent.
*/

pub fn start_backend<F: Fn() -> Deck + Send + 'static>(
    ds: &mut DataSource,
    create_deck: F, //< this function could totally be part of the datasource if i did this?
) -> (
Vec<thread::JoinHandle<()>>, (mpsc::Sender<MessagePacket>, mpsc::Receiver<BackendStatus>)) {
    // Setup the Server thread
    let (tx, rx) = mpsc::channel();
    
    //@note: except this is no good as each user/thread is going to need to be updated.  this either needs to a broadcast OR it needs to be registered with the thread.
    let (client_rx, client_tx) = mpsc::channel();
    
    let handle = thread::spawn(move || {
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
            
            // Task_4: Notify.
            // Let listening threads 
            // know that weve completed a loop 
            client_tx.send(BackendStatus::UpdateComplete);
        }
    });
    
    
    let dealer = thread::spawn(move || {
        loop {
            if let Some(status) = client_rx.recv() {
                match status {
                    BackendStatus::UpdateComplete => {
                        if is_dealer() {
                            // @todo: do something
                        }
                    },
                    _ => {},
                }
            }
        }
    });

    (vec![handle, dealer], tx)
}
