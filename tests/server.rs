//
// Integration tests for our blackjack server
//
use blackjack::{Action, Message, Outcome, Response};
use uuid::Uuid;

//@todo:
//  Figure out logging, when where why how
//  Game should end if all the players go bust or blackjack before the dealer draws
//  Game should end and all active players should win if the dealer busts

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
    let (_, client_tx, response_rx) = blackjack::start_backend();

    // Data required for test.
    let mut game_id = Uuid::nil();
    let mut hand_id = Uuid::nil();
    let mut current_hand_id = Uuid::nil();
    let mut hand_value;
    let mut hand_outcome: Option<Outcome>;

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

    assert_eq!(Some(Outcome::Won(21)), hand_outcome);
}
