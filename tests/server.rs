//
// Integration tests for our blackjack server
//
use blackjack::data_source::*;
use blackjack::types::*;

use std::thread;
//use timer::Timer;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

//@todo:
//  Figure out logging, when where why how
//  Game should end if all the players go bust or blackjack before the dealer draws
//  Game should end and all active players should win if the dealer busts

fn create_loaded_deck() -> blackjack::Deck {
    //@note: For now just going to create a deck with all of the face cards
    //  removed
    vec![
        (blackjack::Suit::Hearts, blackjack::Value::Value(9)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(8)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(7)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(6)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(5)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(4)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(3)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(2)),
        (blackjack::Suit::Hearts, blackjack::Value::Value(1)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(9)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(8)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(7)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(6)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(5)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(4)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(3)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(2)),
        (blackjack::Suit::Diamonds, blackjack::Value::Value(1)),
        (blackjack::Suit::Spades, blackjack::Value::Value(9)),
        (blackjack::Suit::Spades, blackjack::Value::Value(8)),
        (blackjack::Suit::Spades, blackjack::Value::Value(7)),
        (blackjack::Suit::Spades, blackjack::Value::Value(6)),
        (blackjack::Suit::Spades, blackjack::Value::Value(5)),
        (blackjack::Suit::Spades, blackjack::Value::Value(4)),
        (blackjack::Suit::Spades, blackjack::Value::Value(3)),
        (blackjack::Suit::Spades, blackjack::Value::Value(2)),
        (blackjack::Suit::Spades, blackjack::Value::Value(1)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(9)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(8)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(7)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(6)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(5)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(4)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(3)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(2)),
        (blackjack::Suit::Clubs, blackjack::Value::Value(1)),
    ]
}

#[derive(Debug)]
enum Message {
    //@todo: Consolidate these into like an AddResource or something?
    AddGame(oneshot::Sender<Uuid>),
    AddPlayer(Uuid /*game_id*/, oneshot::Sender<Uuid>),
    AddHandAction(
        Uuid, /*hand_id*/
        blackjack::Action,
        oneshot::Sender<Uuid>,
    ),
    StartGame(Uuid /*game_id*/),
    IsGameComplete(Uuid /*game_id*/, oneshot::Sender<bool>),
    IsCurrentHand(Uuid /*hand_id*/, oneshot::Sender<bool>),
    GetHandValue(Uuid /*hand_id*/, oneshot::Sender<u8>),
    GetHandOutcome(
        Uuid, /*hand_id*/
        oneshot::Sender<Option<blackjack::Outcome>>,
    ),
    //Update,
}

fn process(_tx: mpsc::Sender<Message>, mut rx: mpsc::Receiver<Message>) {
    let mut ds = DataSource::default();

    /*
    // Once a second we want to process the simulation so hit it with an Message::Update,
    // consolidates the processing to be completely event based.
    let timer = timer::Timer::new();
    timer.schedule_repeating(chrono::Duration::seconds(60), move || {
        let _ignored = tx.send(Message::Update);
    });
    */

    loop {
        if let Ok(m) = rx.try_recv() {
            match m {
                Message::AddGame(responder_tx) => {
                    let game_id = ds.add_game();
                    responder_tx.send(game_id).unwrap();
                }
                Message::AddPlayer(game_id, responder_tx) => {
                    let player_id = ds.add_player(game_id);
                    responder_tx.send(player_id).unwrap();
                }
                Message::AddHandAction(hand_id, action, responder_tx) => {
                    ds.add_action(hand_id, action);
                    let new_uuid = Uuid::new_v4();
                    responder_tx.send(new_uuid).unwrap();
                }
                Message::StartGame(game_id) => {
                    ds.start_game(game_id);
                }
                Message::IsGameComplete(game_id, responder_tx) => {
                    let is_game_complete =
                        blackjack::is_game_complete(game_id, &ds.hands, &ds.hand_states);
                    responder_tx.send(is_game_complete).unwrap();
                }
                Message::IsCurrentHand(hand_id, responder_tx) => {
                    let is_current_hand = false;
                    responder_tx.send(is_current_hand).unwrap();
                }
                Message::GetHandValue(hand_id, responder_tx) => {
                    let hand_value =
                        blackjack::get_hand_value(hand_id, &ds.hands, &ds.allocations, &ds.decks);
                    responder_tx.send(hand_value).unwrap();
                }
                Message::GetHandOutcome(hand_id, responder_tx) => {
                    let hand_outcome = blackjack::get_hand_outcome(hand_id, &ds.outcomes);
                    responder_tx.send(hand_outcome).unwrap()
                }
            }
        } else {
            // I guess there was no message, if there are any actions we should update them.
            ds.process_hit_actions();
            ds.process_hold_actions();
            ds.resolve_turn();
        }
    }
}

async fn create_game(tx: &mpsc::Sender<Message>) -> Uuid {
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Message::AddGame(resp_tx)).await.unwrap();
    resp_rx.await.unwrap()
}

async fn add_player(tx: &mpsc::Sender<Message>, game_id: Uuid) -> Uuid {
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Message::AddPlayer(game_id, resp_tx)).await.unwrap();
    resp_rx.await.unwrap()
}

async fn is_game_complete(tx: &mpsc::Sender<Message>, game_id: Uuid) -> bool {
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Message::IsGameComplete(game_id, resp_tx))
        .await
        .unwrap();
    resp_rx.await.unwrap()
}

async fn is_current_hand(tx: &mpsc::Sender<Message>, hand_id: Uuid) -> bool {
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Message::IsCurrentHand(hand_id, resp_tx))
        .await
        .unwrap();
    resp_rx.await.unwrap()
}

async fn get_hand_value(tx: &mpsc::Sender<Message>, hand_id: Uuid) -> u8 {
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Message::GetHandValue(hand_id, resp_tx))
        .await
        .unwrap();
    resp_rx.await.unwrap()
}

async fn add_hand_action(
    tx: &mpsc::Sender<Message>,
    hand_id: Uuid,
    action: blackjack::Action,
) -> Uuid {
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Message::AddHandAction(hand_id, action, resp_tx))
        .await
        .unwrap();
    resp_rx.await.unwrap()
}

async fn get_hand_outcome(tx: &mpsc::Sender<Message>, hand_id: Uuid) -> Option<blackjack::Outcome> {
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Message::GetHandOutcome(hand_id, resp_tx))
        .await
        .unwrap();
    resp_rx.await.unwrap()
}

// import our lib and setup a game
#[tokio::test]
async fn can_play_a_simple_game() {
    // Setup the Server thread
    let (tx, rx) = mpsc::channel(32);
    let client_tx = tx.clone();
    let _handle = thread::spawn(move || process(tx, rx));

    // Create the game
    let game_id = create_game(&client_tx).await;

    // ds.set_deck(game_id, create_loaded_deck());

    // Create a player and sit him at the table.
    let hand_id = add_player(&client_tx, game_id).await;

    // Start the game simulation
    client_tx.send(Message::StartGame(game_id)).await.unwrap();

    // loop while the game is active and post the players actions.
    // @todo: extract the dealers actions from here, they shouldnt be here.
    while !is_game_complete(&client_tx, game_id).await {
        if is_current_hand(&client_tx, hand_id).await {
            add_hand_action(&client_tx, hand_id, Action::Hit).await;
        } else {
            // This is the dealers turn.
            let hand_value = get_hand_value(&client_tx, hand_id).await;
            if hand_value > 17 {
                add_hand_action(&client_tx, hand_id, Action::Hold).await;
            } else {
                add_hand_action(&client_tx, hand_id, Action::Hit).await;
            }
        }
    }

    assert_eq!(
        Some(blackjack::Outcome::Won(21)),
        get_hand_outcome(&client_tx, hand_id).await
    );
}
