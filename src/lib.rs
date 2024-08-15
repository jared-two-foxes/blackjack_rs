pub mod data_source;
pub mod types;
pub mod utils;

pub use data_source::DataSource;
pub use types::*;
pub use utils::*;

use std::sync::mpsc;
use uuid::Uuid;

//@todo: Now we need to define a unified repsonse message.
pub enum Resource {
    Game,
    Player,
    HandAction,
}

pub enum Response {
    StatusOk,
    AddResource(Resource, Uuid),
    Hand(Uuid),
    HandValue(u8),
    HandOutcome(Option<Outcome>),
    Failed,
}

#[derive(Debug)]
pub enum Message {
    //@todo: Consolidate these into like an AddResource or something?
    CreateGame,
    AddPlayer(Uuid /*game_id*/),
    AddHandAction(Uuid /*hand_id*/, Action),
    StartGame(Uuid /*game_id*/),
    GetCurrentHand(Uuid /*game_id*/),
    GetHandValue(Uuid /*hand_id*/),
    GetHandOutcome(Uuid /*hand_id*/),
}

pub fn process(
    _tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
    response_tx: mpsc::Sender<Response>,
    deck: Deck,
) {
    let mut ds = DataSource::default();

    loop {
        if let Ok(m) = rx.try_recv() {
            let response = match m {
                Message::CreateGame => {
                    println!("server: Received Create Game Message");
                    //@todo: add_game should probably take the deck to use.
                    let game_id = ds.add_game();
                    ds.set_deck(game_id, deck.clone());
                    Response::AddResource(Resource::Game, game_id)
                }
                Message::AddPlayer(game_id) => {
                    println!("server: AddPlayer");
                    let player_id = ds.add_player(game_id);
                    Response::AddResource(Resource::Player, player_id)
                }
                Message::AddHandAction(hand_id, action) => {
                    println!("server: AddHandAction");
                    ds.add_action(hand_id, action);
                    let new_uuid = Uuid::new_v4();
                    Response::AddResource(Resource::HandAction, new_uuid)
                }
                Message::StartGame(game_id) => {
                    println!("server: StartGame");
                    ds.start_game(game_id);
                    Response::StatusOk
                }
                Message::GetCurrentHand(game_id) => {
                    println!("server: GetCurrentHand");
                    let hand_id = get_active_hand(game_id, &ds.active_hands, &ds.hands);
                    match hand_id {
                        Some(uid) => println!("active hand for {}: {}", game_id, uid),
                        _ => println!("No active hand for given game_id {}", game_id),
                    };
                    hand_id.map_or(Response::Failed, Response::Hand)
                }
                Message::GetHandValue(hand_id) => {
                    println!("server: GetHandValue");
                    let hand_value = get_hand_value(hand_id, &ds.hands, &ds.allocations, &ds.decks);
                    Response::HandValue(hand_value)
                }
                Message::GetHandOutcome(hand_id) => {
                    println!("server: GetHandOutcome");
                    let hand_outcome = get_hand_outcome(hand_id, &ds.outcomes);
                    Response::HandOutcome(hand_outcome)
                }
            };
            response_tx.send(response).unwrap();
        }

        if !ds.actions.is_empty() {
            // I guess there was no message, if there are any actions we should update them.
            ds.process_hit_actions();
            ds.process_hold_actions();
            ds.resolve_turn();
        }
    }
}
