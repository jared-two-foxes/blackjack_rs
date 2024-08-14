//
// Integration tests for our blackjack server
//
use blackjack::data_source::*;
use blackjack::types::*;

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
    Failed,
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
}

fn process(
    _tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
    response_tx: mpsc::Sender<Response>,
) {
    let mut ds = DataSource::default();

    loop {
        if let Ok(m) = rx.try_recv() {
            let response = match m {
                Message::CreateGame => {
                    println!("server: Received Create Game Message");
                    //@todo: add_game should probably take the deck to use.
                    let game_id = ds.add_game();
                    ds.set_deck(game_id, create_loaded_deck());
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
                    let hand_id = blackjack::get_active_hand(game_id, &ds.active_hands, &ds.hands);
                    match hand_id {
                        Some(uid) => println!("active hand for {}: {}", game_id, uid),
                        _ => println!("No active hand for given game_id {}", game_id),
                    };
                    hand_id.map_or(Response::Failed, Response::Hand)
                }
                Message::GetHandValue(hand_id) => {
                    println!("server: GetHandValue");
                    let hand_value =
                        blackjack::get_hand_value(hand_id, &ds.hands, &ds.allocations, &ds.decks);
                    Response::HandValue(hand_value)
                }
                Message::GetHandOutcome(hand_id) => {
                    println!("server: GetHandOutcome");
                    let hand_outcome = blackjack::get_hand_outcome(hand_id, &ds.outcomes);
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
    let _handle = thread::spawn(move || process(tx, rx, response_tx));

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
