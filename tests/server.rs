//
// Integration tests for our blackjack server
//
use blackjack::Outcome;
//use flexi_logger::Logger;
//use log::info;

//@todo:
//  Figure out logging, when where why how
//  Game should end if all the players go bust or blackjack before the dealer draws
//  Game should end and all active players should win if the dealer busts

mod test_framework {

    use blackjack::{Action, Card, CardValue, Deck, Message, Outcome, Resource, Response, Suit};
    use log::{error, info};
    use std::sync::mpsc;

    pub fn create_loaded_deck() -> Deck {
        //@note: For now just going to create a deck with all of the face cards
        //  removed
        vec![
            Card::new(Suit::Hearts, CardValue::Value(9)),
            Card::new(Suit::Hearts, CardValue::Value(8)),
            Card::new(Suit::Hearts, CardValue::Value(7)),
            Card::new(Suit::Hearts, CardValue::Value(6)),
            Card::new(Suit::Hearts, CardValue::Value(5)),
            Card::new(Suit::Hearts, CardValue::Value(4)),
            Card::new(Suit::Hearts, CardValue::Value(3)),
            Card::new(Suit::Hearts, CardValue::Value(2)),
            Card::new(Suit::Hearts, CardValue::Value(1)),
            Card::new(Suit::Diamonds, CardValue::Value(9)),
            Card::new(Suit::Diamonds, CardValue::Value(8)),
            Card::new(Suit::Diamonds, CardValue::Value(7)),
            Card::new(Suit::Diamonds, CardValue::Value(6)),
            Card::new(Suit::Diamonds, CardValue::Value(5)),
            Card::new(Suit::Diamonds, CardValue::Value(4)),
            Card::new(Suit::Diamonds, CardValue::Value(3)),
            Card::new(Suit::Diamonds, CardValue::Value(2)),
            Card::new(Suit::Diamonds, CardValue::Value(1)),
            Card::new(Suit::Spades, CardValue::Value(9)),
            Card::new(Suit::Spades, CardValue::Value(8)),
            Card::new(Suit::Spades, CardValue::Value(7)),
            Card::new(Suit::Spades, CardValue::Value(6)),
            Card::new(Suit::Spades, CardValue::Value(5)),
            Card::new(Suit::Spades, CardValue::Value(4)),
            Card::new(Suit::Spades, CardValue::Value(3)),
            Card::new(Suit::Spades, CardValue::Value(2)),
            Card::new(Suit::Spades, CardValue::Value(1)),
            Card::new(Suit::Clubs, CardValue::Value(9)),
            Card::new(Suit::Clubs, CardValue::Value(8)),
            Card::new(Suit::Clubs, CardValue::Value(7)),
            Card::new(Suit::Clubs, CardValue::Value(6)),
            Card::new(Suit::Clubs, CardValue::Value(5)),
            Card::new(Suit::Clubs, CardValue::Value(4)),
            Card::new(Suit::Clubs, CardValue::Value(3)),
            Card::new(Suit::Clubs, CardValue::Value(2)),
            Card::new(Suit::Clubs, CardValue::Value(1)),
        ]
    }

    //@note: Should there be some kind of PickAction state?  Is that what BeginLoop should be?
    #[derive(Clone, Copy)]
    pub enum TestState {
        GetTableList,
        //@note: I guess this should be some kind of create_or_login or something like that?
        //  Also this is more of a join_table or something.
        CreatePlayer(uuid::Uuid /*game_id*/),
        //@note: Should we need this state?  Does it do anything actually interesting?
        //  is it misnamed or should we go straight to GetHandOutcome?
        //  Also this is bad design, this puts the start at the hands of the players, it should
        //  be the server that determines this.
        //BeginLoop(uuid::Uuid /*game_id*/), //< Loop start
        GetHandOutcome(uuid::Uuid /*hand_id*/),
        GetCurrentHand(uuid::Uuid /*game_id*/),
        GetHandValue(uuid::Uuid /*hand_id*/),
        AddAction(uuid::Uuid, blackjack::Action),
    }

    impl TestState {
        pub fn to_message(&self) -> Message {
            match self {
                Self::GetTableList => Message::GetTableList,
                Self::CreatePlayer(game_id) => Message::AddPlayer(*game_id),
                //Self::BeginLoop(game_id) => Message::StartGame(*game_id),
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
        client_tx: mpsc::Sender<blackjack::MessagePacket>,
        response_tx: mpsc::Sender<blackjack::Response>,
    }

    impl StateController {
        pub fn new(
            starting_state: TestState,
            client_tx: mpsc::Sender<blackjack::MessagePacket>,
            response_tx: mpsc::Sender<blackjack::Response>,
        ) -> StateController {
            if let Some(state) = Self::on_enter(&client_tx, starting_state, &response_tx) {
                StateController {
                    current_state: state,
                    client_tx,
                    response_tx,
                }
            } else {
                panic!("Unable to start StateController")
            }
        }

        // This is a function that doesnt take a Self, so it is basically a static function.  Does
        // that belong here or not really?  Should it be elsewhere, I really dont know.
        fn on_enter(
            ctx: &mpsc::Sender<blackjack::MessagePacket>,
            new_state: TestState,
            response_tx: &mpsc::Sender<blackjack::Response>,
        ) -> Option<TestState> {
            let message_packet = blackjack::MessagePacket {
                message: new_state.to_message(),
                response_tx: response_tx.clone(),
            };
            match ctx.send(message_packet) {
                Ok(_) => Some(new_state),
                Err(e) => {
                    error!("TODO: Should do something if this fails to send",);
                    error!("error: {}", e);
                    None
                }
            }
        }

        pub fn set_state(&mut self, new_state: TestState) {
            if let Some(state) = Self::on_enter(&self.client_tx, new_state, &self.response_tx) {
                self.current_state = state;
            }
        }
    }

    pub struct HandController {
        fsm: StateController,
        game_id: uuid::Uuid,
        hand_id: uuid::Uuid,
        current_hand_id: uuid::Uuid,
        hand_outcome: Option<Outcome>,
        response_rx: mpsc::Receiver<Response>,
    }

    impl HandController {
        // Not sure how this should work, we kinda need the game_id to differentiate between "games"
        // that the Controller represents
        pub fn new(
            starting_state: TestState,
            client_tx: mpsc::Sender<blackjack::MessagePacket>,
        ) -> HandController {
            let (response_tx, response_rx) = mpsc::channel();
            HandController {
                fsm: StateController::new(starting_state, client_tx, response_tx),
                game_id: uuid::Uuid::nil(),
                hand_id: uuid::Uuid::nil(),
                current_hand_id: uuid::Uuid::nil(),
                hand_outcome: None,
                response_rx,
            }
        }

        //@todo: Should maybe be called a GameController instead?
        // Returns false if we're done with this Game
        pub fn process(&mut self) -> bool {
            let received = self.response_rx.try_recv();
            if let Ok(response) = received {
                match response {
                    Response::Failed => {
                        //@note: Well shit something bad happened and I dont know what to do about it!
                        unimplemented!();
                    }
                    Response::StatusOk => {
                        // We can get a StatusOk in response to a StartGame message.
                        self.fsm.set_state(TestState::GetHandOutcome(self.hand_id));
                    }
                    Response::AddResource(resource, uid) => {
                        //@note: this will depend on what state we are in! Either we are attempting
                        //  to add a new game, or a new player
                        match resource {
                            Resource::Game => {
                                self.game_id = uid;
                                info!("client: game_id={}", self.game_id);
                                self.fsm.set_state(TestState::CreatePlayer(self.game_id));
                            }
                            Resource::Player => {
                                self.hand_id = uid;
                                info!("client: hand_id={}", self.hand_id);
                                unimplemented!();
                                //self.fsm.set_state(TestState::BeginLoop(self.game_id));
                            }
                            Resource::HandAction => {
                                //@note: So we've sent our action, start the loop again for the next player
                                self.fsm.set_state(TestState::GetHandOutcome(self.hand_id));
                            }
                        }
                    }
                    Response::TableList(tables) => {
                        //@note: tables is a list of tables, not sure if we need to be checking here if
                        //the 'first' table is open or not.
                        self.game_id = *tables.first().unwrap();
                    }
                    Response::HandOutcome(outcome) => {
                        // If there is some hand outcome for the test hand then the test is finished
                        match outcome {
                            Some(o) => {
                                self.hand_outcome = Some(o);
                                return true;
                            }
                            None => {
                                self.fsm.set_state(TestState::GetCurrentHand(self.game_id));
                            }
                        }
                    }
                    Response::Hand(uid) => {
                        self.current_hand_id = uid;
                        info!("client: Current Hand={}", self.current_hand_id);
                        self.fsm
                            .set_state(TestState::GetHandValue(self.current_hand_id));
                    }
                    Response::HandValue(value) => {
                        info!("client: Hand Value={}", value);
                        if value > 17 {
                            self.fsm.set_state(TestState::AddAction(
                                self.current_hand_id,
                                Action::Hold,
                            ));
                        } else {
                            self.fsm
                                .set_state(TestState::AddAction(self.current_hand_id, Action::Hit));
                        }
                    }
                }
            }
            false
        }

        pub fn get_hand_outcome(&self) -> Option<Outcome> {
            self.hand_outcome
        }
    }
}

#[test]
fn can_play_a_simple_game() {
    use test_framework::{HandController, TestState};

    /*Logger::try_with_str("trace")
    .unwrap()
    .log_to_stdout()
    .start()
    .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));*/

    let (_, client_tx) = blackjack::start_backend(test_framework::create_loaded_deck);
    let mut controller = HandController::new(TestState::GetTableList, client_tx);

    loop {
        if controller.process() {
            break;
        }
    }

    assert_eq!(Some(Outcome::Lost(19)), controller.get_hand_outcome());
}

#[test]
fn player_can_sit_at_multiple_tables() {
    use test_framework::{HandController, TestState};
    /*Logger::try_with_str("trace")
    .unwrap()
    .log_to_stdout()
    .start()
    .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));*/

    let (_, client_tx) = blackjack::start_backend(test_framework::create_loaded_deck);
    let mut hand_one = HandController::new(TestState::GetTableList, client_tx.clone());
    let mut hand_two = HandController::new(TestState::GetTableList, client_tx);

    loop {
        //@todo: Determine how to check if both hands are complete rather than have it as a
        //response of process
        if hand_one.process() {
            break;
        }
        if hand_two.process() {
            break;
        }
    }

    assert_eq!(Some(Outcome::Lost(19)), hand_one.get_hand_outcome());
    assert_eq!(Some(Outcome::Lost(19)), hand_two.get_hand_outcome());
}
