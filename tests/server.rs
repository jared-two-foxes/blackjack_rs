//
// Integration tests for our blackjack server
//
use blackjack::data_source::*;
use blackjack::types::*;

use std::thread;
//use timer::Timer;
use tokio::sync::mpsc;
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

//@todo: Now we need to define a unified repsonse message.
enum Resource {
    Game,
    Player,
    HandAction,
}

enum Response {
    StatusOk,
    AddResource(Resource, Uuid),
    Hand(Uuid),
    HandValue(u8),
    HandOutcome(Option<blackjack::Outcome>),
}

#[derive(Debug)]
enum Message {
    //@todo: Consolidate these into like an AddResource or something?
    CreateGame(mpsc::Sender<Response>),
    AddPlayer(Uuid /*game_id*/, mpsc::Sender<Response>),
    AddHandAction(
        Uuid, /*hand_id*/
        blackjack::Action,
        mpsc::Sender<Response>,
    ),
    StartGame(Uuid /*game_id*/, mpsc::Sender<Response>),
    GetCurrentHand(Uuid /*game_id*/, mpsc::Sender<Response>),
    GetHandValue(Uuid /*hand_id*/, mpsc::Sender<Response>),
    GetHandOutcome(Uuid /*hand_id*/, mpsc::Sender<Response>),
    //Update,
}

async fn process(_tx: mpsc::Sender<Message>, mut rx: mpsc::Receiver<Message>) {
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
                Message::CreateGame(responder_tx) => {
                    //@todo: add_game should probably take the deck to use.
                    let game_id = ds.add_game();
                    ds.set_deck(game_id, create_loaded_deck());
                    responder_tx
                        .send(Response::AddResource(Resource::Game, game_id))
                        .await
                        .unwrap()
                }
                Message::AddPlayer(game_id, responder_tx) => {
                    let player_id = ds.add_player(game_id);
                    responder_tx
                        .send(Response::AddResource(Resource::Player, player_id))
                        .await
                        .unwrap();
                }
                Message::AddHandAction(hand_id, action, responder_tx) => {
                    ds.add_action(hand_id, action);
                    let new_uuid = Uuid::new_v4();
                    responder_tx
                        .send(Response::AddResource(Resource::HandAction, new_uuid))
                        .await
                        .unwrap();
                }
                Message::StartGame(game_id, responder_tx) => {
                    ds.start_game(game_id);
                    responder_tx.send(Response::StatusOk).await.unwrap();
                }
                Message::GetCurrentHand(_game_id, responder_tx) => {
                    //@todo: Pull the actual current hand for the passed game.
                    responder_tx
                        .send(Response::Hand(Uuid::nil()))
                        .await
                        .unwrap();
                }
                Message::GetHandValue(hand_id, responder_tx) => {
                    let hand_value =
                        blackjack::get_hand_value(hand_id, &ds.hands, &ds.allocations, &ds.decks);
                    responder_tx
                        .send(Response::HandValue(hand_value))
                        .await
                        .unwrap();
                }
                Message::GetHandOutcome(hand_id, responder_tx) => {
                    let hand_outcome = blackjack::get_hand_outcome(hand_id, &ds.outcomes);
                    responder_tx
                        .send(Response::HandOutcome(hand_outcome))
                        .await
                        .unwrap()
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

async fn create_game(tx: &mpsc::Sender<Message>, resp_tx: mpsc::Sender<Response>) {
    tx.send(Message::CreateGame(resp_tx)).await.unwrap();
}

async fn add_player(tx: &mpsc::Sender<Message>, game_id: Uuid, resp_tx: mpsc::Sender<Response>) {
    tx.send(Message::AddPlayer(game_id, resp_tx)).await.unwrap();
}

async fn get_hand_outcome(
    tx: &mpsc::Sender<Message>,
    hand_id: Uuid,
    resp_tx: mpsc::Sender<Response>,
) {
    tx.send(Message::GetHandOutcome(hand_id, resp_tx))
        .await
        .unwrap();
}

async fn get_current_hand(
    tx: &mpsc::Sender<Message>,
    hand_id: Uuid,
    resp_tx: mpsc::Sender<Response>,
) {
    tx.send(Message::GetCurrentHand(hand_id, resp_tx))
        .await
        .unwrap();
}

async fn get_hand_value(
    tx: &mpsc::Sender<Message>,
    hand_id: Uuid,
    resp_tx: mpsc::Sender<Response>,
) {
    tx.send(Message::GetHandValue(hand_id, resp_tx))
        .await
        .unwrap();
}

async fn add_hand_action(
    tx: &mpsc::Sender<Message>,
    hand_id: Uuid,
    action: blackjack::Action,
    resp_tx: mpsc::Sender<Response>,
) {
    tx.send(Message::AddHandAction(hand_id, action, resp_tx))
        .await
        .unwrap();
}

enum TestState {
    CreateGame,
    CreatePlayer,
    Update, //< Loop start
    GetHandOutcome,
    GetCurrentHand,
    GetHandValue,
    AddAction,
}

// import our lib and setup a game
#[tokio::test]
async fn can_play_a_simple_game() {
    // Setup the Server thread
    let (tx, rx) = mpsc::channel(32);
    let client_tx = tx.clone();
    let _handle = thread::spawn(move || process(tx, rx));

    let (tx2, mut rx2) = mpsc::channel(32);

    // Data required for test.
    let mut game_id = Uuid::nil();
    let mut hand_id = Uuid::nil();
    let mut current_hand_id = Uuid::nil();
    let mut hand_value = 0u8;
    let mut hand_outcome: Option<blackjack::Outcome>;

    create_game(&client_tx, tx2.clone()).await;
    let mut state = TestState::CreateGame;

    loop {
        if let Ok(response) = rx2.try_recv() {
            //@todo: let this match return a (message, new_state) pair and then we can relocate the
            //  await from the following functions here.
            match state {
                TestState::CreateGame => {
                    if let Response::AddResource(_, uid) = response {
                        game_id = uid;
                    }
                    add_player(&client_tx, game_id, tx2.clone()).await;
                    state = TestState::CreatePlayer;
                }
                TestState::CreatePlayer => {
                    if let Response::AddResource(_, uid) = response {
                        hand_id = uid;
                    }
                    client_tx
                        .send(Message::StartGame(game_id, tx2.clone()))
                        .await
                        .unwrap();
                    state = TestState::Update;
                }
                TestState::Update => {
                    get_hand_outcome(&client_tx, game_id, tx2.clone()).await;
                    state = TestState::GetHandOutcome;
                }
                TestState::GetHandOutcome => {
                    if let Response::HandOutcome(outcome) = response {
                        hand_outcome = outcome;
                        if hand_outcome.is_some() {
                            break;
                        }
                    }
                    get_current_hand(&client_tx, hand_id, tx2.clone()).await;
                    state = TestState::GetCurrentHand;
                }
                TestState::GetCurrentHand => {
                    if let Response::Hand(uid) = response {
                        current_hand_id = uid;
                    }
                    get_hand_value(&client_tx, hand_id, tx2.clone()).await;
                    state = TestState::GetHandValue;
                }
                TestState::GetHandValue => {
                    if let Response::HandValue(value) = response {
                        hand_value = value;
                    }

                    // @todo: extract the dealers actions from here, they shouldnt be here.
                    if current_hand_id == hand_id {
                        add_hand_action(&client_tx, hand_id, Action::Hit, tx2.clone()).await;
                        state = TestState::AddAction;
                    } else if hand_value > 17 {
                        add_hand_action(&client_tx, hand_id, Action::Hold, tx2.clone()).await;
                    } else {
                        add_hand_action(&client_tx, hand_id, Action::Hit, tx2.clone()).await;
                    }
                }
                TestState::AddAction => {
                    // And loop back to the start
                    state = TestState::Update;
                }
            }
        }
    }

    assert_eq!(Some(blackjack::Outcome::Won(21)), hand_outcome);
}
