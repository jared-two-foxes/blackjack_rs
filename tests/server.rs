//
// Integration tests for our blackjack server
//
use blackjack::{
    types::{Suit, Value},
    Action, Message, Response,
};

use std::sync::mpsc;
use std::thread;
use uuid::Uuid;

//@todo:
//  Figure out logging, when where why how
//  Game should end if all the players go bust or blackjack before the dealer draws
//  Game should end and all active players should win if the dealer busts

fn create_loaded_deck() -> blackjack::Deck {
    //@note: For now just going to create a deck with all of the face cards
    //  removed
    vec![
        (Suit::Hearts, Value::Value(9)),
        (Suit::Hearts, Value::Value(8)),
        (Suit::Hearts, Value::Value(7)),
        (Suit::Hearts, Value::Value(6)),
        (Suit::Hearts, Value::Value(5)),
        (Suit::Hearts, Value::Value(4)),
        (Suit::Hearts, Value::Value(3)),
        (Suit::Hearts, Value::Value(2)),
        (Suit::Hearts, Value::Value(1)),
        (Suit::Diamonds, Value::Value(9)),
        (Suit::Diamonds, Value::Value(8)),
        (Suit::Diamonds, Value::Value(7)),
        (Suit::Diamonds, Value::Value(6)),
        (Suit::Diamonds, Value::Value(5)),
        (Suit::Diamonds, Value::Value(4)),
        (Suit::Diamonds, Value::Value(3)),
        (Suit::Diamonds, Value::Value(2)),
        (Suit::Diamonds, Value::Value(1)),
        (Suit::Spades, Value::Value(9)),
        (Suit::Spades, Value::Value(8)),
        (Suit::Spades, Value::Value(7)),
        (Suit::Spades, Value::Value(6)),
        (Suit::Spades, Value::Value(5)),
        (Suit::Spades, Value::Value(4)),
        (Suit::Spades, Value::Value(3)),
        (Suit::Spades, Value::Value(2)),
        (Suit::Spades, Value::Value(1)),
        (Suit::Clubs, Value::Value(9)),
        (Suit::Clubs, Value::Value(8)),
        (Suit::Clubs, Value::Value(7)),
        (Suit::Clubs, Value::Value(6)),
        (Suit::Clubs, Value::Value(5)),
        (Suit::Clubs, Value::Value(4)),
        (Suit::Clubs, Value::Value(3)),
        (Suit::Clubs, Value::Value(2)),
        (Suit::Clubs, Value::Value(1)),
    ]
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
#[test]
fn can_play_a_simple_game() {
    // Setup the Server thread
    let (tx, rx) = mpsc::channel();
    let (response_tx, response_rx) = mpsc::channel();
    let client_tx = tx.clone();
    let _handle =
        thread::spawn(move || blackjack::process(tx, rx, response_tx, create_loaded_deck()));

    // Data required for test.
    let mut game_id = Uuid::nil();
    let mut hand_id = Uuid::nil();
    let mut current_hand_id = Uuid::nil();
    let mut hand_value;
    let mut hand_outcome: Option<blackjack::Outcome>;

    client_tx.send(Message::CreateGame).unwrap();
    let mut state = TestState::CreateGame;

    loop {
        let received = response_rx.try_recv();
        let message = match state {
            TestState::CreateGame => {
                if let Ok(Response::AddResource(_, uid)) = received {
                    game_id = uid;
                    println!("client: game_id={}", game_id);
                    state = TestState::CreatePlayer;
                    Some(Message::AddPlayer(game_id))
                } else {
                    None
                }
            }
            TestState::CreatePlayer => {
                if let Ok(Response::AddResource(_, uid)) = received {
                    hand_id = uid;
                    println!("client: hand_id={}", hand_id);
                    state = TestState::BeginLoop;
                    Some(Message::StartGame(game_id))
                } else {
                    None
                }
            }
            TestState::BeginLoop => {
                println!("client: Begining Loop");
                state = TestState::GetHandOutcome;
                Some(Message::GetHandOutcome(hand_id))
            }
            TestState::GetHandOutcome => {
                if let Ok(Response::HandOutcome(outcome)) = received {
                    println!("client: Retrieved Hand Outcome");
                    hand_outcome = outcome;
                    // If there is some hand outcome then the test is finished, bail
                    if hand_outcome.is_some() {
                        println!("client: Found some outcome");
                        break;
                    }
                    state = TestState::GetCurrentHand;
                    Some(Message::GetCurrentHand(game_id))
                } else {
                    None
                }
            }
            TestState::GetCurrentHand => {
                if let Ok(Response::Hand(uid)) = received {
                    current_hand_id = uid;
                    println!("client: Current Hand={}", current_hand_id);
                    state = TestState::GetHandValue;
                    Some(Message::GetHandValue(current_hand_id))
                } else {
                    None
                }
            }
            TestState::GetHandValue => {
                if let Ok(Response::HandValue(value)) = received {
                    println!("client: Hand Value={}", value);
                    hand_value = value;
                    // @todo: extract the dealers actions from here, they shouldnt be here.
                    let m = if hand_value > 17 {
                        Message::AddHandAction(current_hand_id, Action::Hold)
                    } else {
                        Message::AddHandAction(current_hand_id, Action::Hit)
                    };
                    state = TestState::AddAction;
                    Some(m)
                } else {
                    None
                }
            }
            TestState::AddAction => {
                // And loop back to the start
                state = TestState::BeginLoop;
                None
            }
        };

        if let Some(msg) = message {
            client_tx.send(msg).unwrap();
        }
    }

    assert_eq!(Some(blackjack::Outcome::Won(21)), hand_outcome);
}
