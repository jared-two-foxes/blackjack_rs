pub mod backend;
pub mod data_source;
pub mod types;
pub mod utils;

pub use backend::{process, Message, Resource, Response};
pub use types::{Action, Deck, Outcome};

use std::sync::mpsc;
use std::thread;
use utils::create_loaded_deck;

//@todo: pass the deck creation function here.
pub fn start_backend() -> (
    thread::JoinHandle<()>,
    mpsc::Sender<Message>,
    mpsc::Receiver<Response>,
) {
    // Setup the Server thread
    let (tx, rx) = mpsc::channel();
    let (response_tx, response_rx) = mpsc::channel();
    let backend_tx = tx.clone();
    let handle = thread::spawn(move || process(backend_tx, rx, response_tx, create_loaded_deck()));

    (handle, tx, response_rx)
}
