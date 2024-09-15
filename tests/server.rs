//
// Integration tests for our blackjack server
//
use blackjack::{Action, Outcome, Resource, Response};

//@todo:
//  Figure out logging, when where why how
//  Game should end if all the players go bust or blackjack before the dealer draws
//  Game should end and all active players should win if the dealer busts

mod test_framework {

    use blackjack::Message;
    use std::sync::mpsc;

    //@note: Should there be some kind of PickAction state?  Is that what BeginLoop should be?
    #[derive(Clone, Copy)]
    pub enum TestState {
        //@note: This state 'CreateGame' doesnt really make sense for something that the
        //  client would interact with.  It should be invisible to the end user.
        CreateGame,
        //@note: I guess this should be some kind of create_or_login or something like that?
        //  Also this is more of a join_table or something.
        CreatePlayer(uuid::Uuid /*game_id*/),
        //@note: Should we need this state?  Does it do anything actually interesting?
        //  is it misnamed or should we go straight to GetHandOutcome?
        //  Also this is bad design, this puts the start at the hands of the players, it should
        //  be the server that determines this.
        BeginLoop(uuid::Uuid /*game_id*/), //< Loop start
        GetHandOutcome(uuid::Uuid /*hand_id*/),
        GetCurrentHand(uuid::Uuid /*game_id*/),
        GetHandValue(uuid::Uuid /*hand_id*/),
        AddAction(uuid::Uuid, blackjack::Action),
    }

    impl TestState {
        pub fn to_message(&self) -> Message {
            match self {
                Self::CreateGame => Message::CreateGame,
                Self::CreatePlayer(game_id) => Message::AddPlayer(*game_id),
                Self::BeginLoop(game_id) => Message::StartGame(*game_id),
                Self::GetHandOutcome(hand_id) => Message::GetHandOutcome(*hand_id),
                Self::GetCurrentHand(game_id) => Message::GetCurrentHand(*game_id),
                Self::GetHandValue(current_hand_id) => Message::GetHandValue(*current_hand_id),
                Self::AddAction(current_hand_id, action) => {
                    Message::AddHandAction(*current_hand_id, *action)
                }
            }
        }
    }

    pub struct StateController {
        pub current_state: TestState,
        client_tx: mpsc::Sender<blackjack::Message>,
    }

    impl StateController {
        pub fn new(
            starting_state: TestState,
            client_tx: mpsc::Sender<blackjack::Message>,
        ) -> StateController {
            if let Some(state) = Self::on_enter(&client_tx, starting_state) {
                StateController {
                    current_state: state,
                    client_tx,
                }
            } else {
                panic!("Unable to start StateController")
            }
        }

        // This is a function that doesnt take a Self, so it is basically a static function.  Does
        // that belong here or not really?  Should it be elsewhere, I really dont know.
        fn on_enter(
            ctx: &mpsc::Sender<blackjack::Message>,
            new_state: TestState,
        ) -> Option<TestState> {
            match ctx.send(new_state.to_message()) {
                Ok(_) => Some(new_state),
                Err(e) => {
                    eprintln!("TODO: Should do something if this fails to send",);
                    eprintln!("error: {}", e);
                    None
                }
            }
        }

        pub fn set_state(&mut self, new_state: TestState) {
            if let Some(state) = Self::on_enter(&self.client_tx, new_state) {
                self.current_state = state;
            }
        }
    }
}

// import our lib and setup a game
#[test]
fn can_play_a_simple_game() {
    use test_framework::{StateController, TestState};

    let (_, client_tx, response_rx) = blackjack::start_backend();

    // Data required for test.
    let mut game_id = uuid::Uuid::nil();
    let mut hand_id = uuid::Uuid::nil();
    let mut current_hand_id = uuid::Uuid::nil();
    let hand_outcome: Option<Outcome>;

    let mut fsm = StateController::new(TestState::CreateGame, client_tx);

    loop {
        let received = response_rx.try_recv();
        if let Ok(response) = received {
            match response {
                Response::Failed => {
                    //@note: Well shit something bad happened and I dont know what to do about it!
                    unimplemented!();
                }
                Response::StatusOk => {
                    // We can get a StatusOk in response to a StartGame message.
                    fsm.set_state(TestState::GetHandOutcome(hand_id));
                }
                Response::AddResource(resource, uid) => {
                    //@note: this will depend on what state we are in! Either we are attempting
                    //  to add a new game, or a new player
                    match resource {
                        Resource::Game => {
                            game_id = uid;
                            println!("client: game_id={}", game_id);
                            fsm.set_state(TestState::CreatePlayer(game_id));
                        }
                        Resource::Player => {
                            hand_id = uid;
                            println!("client: hand_id={}", hand_id);
                            fsm.set_state(TestState::BeginLoop(game_id));
                        }
                        Resource::HandAction => {
                            //@note: So we've sent our action, start the loop again for the next player
                            fsm.set_state(TestState::GetHandOutcome(hand_id));
                        }
                    }
                }
                Response::HandOutcome(outcome) => {
                    // If there is some hand outcome for the test hand then the test is finished
                    match outcome {
                        Some(o) => {
                            hand_outcome = Some(o);
                            break;
                        }
                        None => {
                            fsm.set_state(TestState::GetCurrentHand(game_id));
                        }
                    }
                }
                Response::Hand(uid) => {
                    current_hand_id = uid;
                    println!("client: Current Hand={}", current_hand_id);
                    fsm.set_state(TestState::GetHandValue(current_hand_id));
                }
                Response::HandValue(value) => {
                    println!("client: Hand Value={}", value);
                    if value > 17 {
                        fsm.set_state(TestState::AddAction(current_hand_id, Action::Hold));
                    } else {
                        fsm.set_state(TestState::AddAction(current_hand_id, Action::Hit));
                    }
                }
            }
        }
    }

    assert_eq!(Some(Outcome::Won(21)), hand_outcome);
}
