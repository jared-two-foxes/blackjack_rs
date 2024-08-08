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
    CreateGame,
    AddPlayer(Uuid /*game_id*/),
    AddHandAction(Uuid /*hand_id*/, blackjack::Action),
    StartGame(Uuid /*game_id*/),
    GetCurrentHand(Uuid /*game_id*/),
    GetHandValue(Uuid /*hand_id*/),
    GetHandOutcome(Uuid /*hand_id*/),
    //Update,
}

async fn process(
    _tx: mpsc::Sender<Message>,
    mut rx: mpsc::Receiver<Message>,
    response_tx: mpsc::Sender<Response>,
) {
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
            let response = match m {
                Message::CreateGame => {
                    //@todo: add_game should probably take the deck to use.
                    let game_id = ds.add_game();
                    ds.set_deck(game_id, create_loaded_deck());
                    Response::AddResource(Resource::Game, game_id)
                }
                Message::AddPlayer(game_id) => {
                    let player_id = ds.add_player(game_id);
                    Response::AddResource(Resource::Player, player_id)
                }
                Message::AddHandAction(hand_id, action) => {
                    ds.add_action(hand_id, action);
                    let new_uuid = Uuid::new_v4();
                    Response::AddResource(Resource::HandAction, new_uuid)
                }
                Message::StartGame(game_id) => {
                    ds.start_game(game_id);
                    Response::StatusOk
                }
                Message::GetCurrentHand(_game_id) => {
                    //@todo: Pull the actual current hand for the passed game.
                    Response::Hand(Uuid::nil())
                }
                Message::GetHandValue(hand_id) => {
                    let hand_value =
                        blackjack::get_hand_value(hand_id, &ds.hands, &ds.allocations, &ds.decks);
                    Response::HandValue(hand_value)
                }
                Message::GetHandOutcome(hand_id) => {
                    let hand_outcome = blackjack::get_hand_outcome(hand_id, &ds.outcomes);
                    Response::HandOutcome(hand_outcome)
                }
            };
            //@todo: Is there a way to do this without the await?
            response_tx.send(response).await.unwrap()
        } else {
            // I guess there was no message, if there are any actions we should update them.
            ds.process_hit_actions();
            ds.process_hold_actions();
            ds.resolve_turn();
        }
    }
}

enum TestState {
    CreateGame,
    CreatePlayer,
    BeginLoop, //< Loop start
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
    let (response_tx, mut response_rx) = mpsc::channel(32);
    let client_tx = tx.clone();
    let _handle = thread::spawn(move || process(tx, rx, response_tx));

    // Data required for test.
    let mut game_id = Uuid::nil();
    let mut hand_id = Uuid::nil();
    let mut current_hand_id = Uuid::nil();
    let mut hand_value = 0u8;
    let mut hand_outcome: Option<blackjack::Outcome>;

    client_tx.send(Message::CreateGame).await.unwrap();
    let mut state = TestState::CreateGame;

    loop {
        if let Ok(response) = response_rx.try_recv() {
            //@todo: let this match return a (message, new_state) pair and then we can relocate the
            //  await from the following functions here.
            let message = match state {
                TestState::CreateGame => {
                    if let Response::AddResource(_, uid) = response {
                        game_id = uid;
                    }
                    state = TestState::CreatePlayer;
                    Some(Message::AddPlayer(game_id))
                }
                TestState::CreatePlayer => {
                    if let Response::AddResource(_, uid) = response {
                        hand_id = uid;
                    }
                    state = TestState::BeginLoop;
                    Some(Message::StartGame(game_id))
                }
                TestState::BeginLoop => {
                    state = TestState::GetHandOutcome;
                    Some(Message::GetHandOutcome(hand_id))
                }
                TestState::GetHandOutcome => {
                    if let Response::HandOutcome(outcome) = response {
                        hand_outcome = outcome;
                        // If there is some hand outcome then the test is finished, bail
                        if hand_outcome.is_some() {
                            break;
                        }
                    }
                    state = TestState::GetCurrentHand;
                    Some(Message::GetCurrentHand(game_id))
                }
                TestState::GetCurrentHand => {
                    if let Response::Hand(uid) = response {
                        current_hand_id = uid;
                    }
                    state = TestState::GetHandValue;
                    Some(Message::GetHandValue(hand_id))
                }
                TestState::GetHandValue => {
                    if let Response::HandValue(value) = response {
                        hand_value = value;
                    }
                    // @todo: extract the dealers actions from here, they shouldnt be here.
                    let m = if current_hand_id == hand_id {
                        Message::AddHandAction(hand_id, Action::Hit)
                    } else if hand_value > 17 {
                        Message::AddHandAction(hand_id, Action::Hold)
                    } else {
                        Message::AddHandAction(hand_id, Action::Hit)
                    };
                    state = TestState::AddAction;
                    Some(m)
                }
                TestState::AddAction => {
                    // And loop back to the start
                    state = TestState::BeginLoop;
                    None
                }
            };

            if let Some(msg) = message {
                client_tx.send(msg).await.unwrap();
            }
        }
    }

    assert_eq!(Some(blackjack::Outcome::Won(21)), hand_outcome);
}
