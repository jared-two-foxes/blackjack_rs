use std::collections::HashMap;
use uuid::Uuid;

pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

pub enum Value {
    Ace,
    King,
    Queen,
    Jack,
    Value(u8),
}

pub type Card = (Suit, Value);

#[derive(Clone)]
pub struct Hand {
    pub id: Uuid,
    player: Uuid,
    dealer: Uuid,
}

fn get_dealer(hand_id: Uuid, hands: &[Hand]) -> Uuid {
    hands
        .iter()
        .find(|h| hand_id == h.id)
        .expect("Unable to find Hand")
        .dealer
}

pub fn get_hand_value(
    hand_id: Uuid,
    hands: &[Hand],
    allocations: &[CardAllocation],
    decks: &HashMap<Uuid, Deck>,
) -> u8 {
    let dealer = get_dealer(hand_id, hands);
    let deck = decks.get(&dealer).expect("Unable to find deck");
    let cards = allocations
        .iter()
        .filter(|a| a.hand == hand_id)
        .map(|a| &deck[a.card_idx])
        .collect::<Vec<_>>();
    hand_value(&cards)
}

fn hand_value(cards: &[&Card]) -> u8 {
    let mut ace_count = 0;
    let mut value = cards
        .iter()
        .map(|c| match c.1 {
            Value::Value(v) => v,
            Value::Ace => {
                ace_count += 1;
                11
            }
            _ => 10,
        })
        .sum::<u8>();
    for _ in 0..ace_count {
        if value > 21 {
            value -= 10;
        }
    }
    value
}

pub type Deck = Vec<Card>;

pub fn new_deck() -> Deck {
    vec![
        (Suit::Hearts, Value::Ace),
        (Suit::Hearts, Value::King),
        (Suit::Hearts, Value::Queen),
        (Suit::Hearts, Value::Jack),
        (Suit::Hearts, Value::Value(10)),
        (Suit::Hearts, Value::Value(9)),
        (Suit::Hearts, Value::Value(8)),
        (Suit::Hearts, Value::Value(7)),
        (Suit::Hearts, Value::Value(6)),
        (Suit::Hearts, Value::Value(5)),
        (Suit::Hearts, Value::Value(4)),
        (Suit::Hearts, Value::Value(3)),
        (Suit::Hearts, Value::Value(2)),
        (Suit::Hearts, Value::Value(1)),
        (Suit::Diamonds, Value::Ace),
        (Suit::Diamonds, Value::King),
        (Suit::Diamonds, Value::Queen),
        (Suit::Diamonds, Value::Jack),
        (Suit::Diamonds, Value::Value(10)),
        (Suit::Diamonds, Value::Value(9)),
        (Suit::Diamonds, Value::Value(8)),
        (Suit::Diamonds, Value::Value(7)),
        (Suit::Diamonds, Value::Value(6)),
        (Suit::Diamonds, Value::Value(5)),
        (Suit::Diamonds, Value::Value(4)),
        (Suit::Diamonds, Value::Value(3)),
        (Suit::Diamonds, Value::Value(2)),
        (Suit::Diamonds, Value::Value(1)),
        (Suit::Clubs, Value::Ace),
        (Suit::Clubs, Value::King),
        (Suit::Clubs, Value::Queen),
        (Suit::Clubs, Value::Jack),
        (Suit::Clubs, Value::Value(10)),
        (Suit::Clubs, Value::Value(9)),
        (Suit::Clubs, Value::Value(8)),
        (Suit::Clubs, Value::Value(7)),
        (Suit::Clubs, Value::Value(6)),
        (Suit::Clubs, Value::Value(5)),
        (Suit::Clubs, Value::Value(4)),
        (Suit::Clubs, Value::Value(3)),
        (Suit::Clubs, Value::Value(2)),
        (Suit::Clubs, Value::Value(1)),
        (Suit::Spades, Value::Ace),
        (Suit::Spades, Value::King),
        (Suit::Spades, Value::Queen),
        (Suit::Spades, Value::Jack),
        (Suit::Spades, Value::Value(10)),
        (Suit::Spades, Value::Value(9)),
        (Suit::Spades, Value::Value(8)),
        (Suit::Spades, Value::Value(7)),
        (Suit::Spades, Value::Value(6)),
        (Suit::Spades, Value::Value(5)),
        (Suit::Spades, Value::Value(4)),
        (Suit::Spades, Value::Value(3)),
        (Suit::Spades, Value::Value(2)),
        (Suit::Spades, Value::Value(1)),
    ]
}

pub struct CardAllocation {
    pub hand: Uuid,
    pub dealer: Uuid, //< this is also dealer's uuid since that is how we identify specific decks.
    pub card_idx: usize,
}

// @todo: I've seen this Hold referenced as "Stand" which I guess makes more sense?
pub enum Action {
    Hit,
    Hold,
}

pub enum State {
    Active, // @todo: When representing hand states in the DataSource this should not be included
    // as it will mess with the game complete calaculation.
    Holding(u8),
    Bust(u8),
    BlackJack,
}

//pair mapping hand to an action
pub type HandAction = (Uuid, Action);

// Pair mapping hand to its current state.
// @todo: This is internal only, maybe we should replace this dealer uuid with a direct index back
// into the array to avoid the O(n) finds required to get the dealers HandState when calculating
// the HandOutcome.
pub type HandState = (Uuid /*this*/, Uuid /*dealer*/, State);

pub fn is_hand_active(hand_id: Uuid, hand_states: &[HandState]) -> bool {
    hand_states.iter().find(|&hs| hs.0 == hand_id).is_none()
}

#[derive(Default)]
pub struct DataSource {
    pub hands: Vec<Hand>,
    pub decks: HashMap<Uuid, Deck>, // map of game_id to Deck for a given game
    pub allocations: Vec<CardAllocation>,
    pub hand_states: Vec<HandState>,
    pub actions: Vec<HandAction>,
    pub outcomes: Vec<HandOutcome>,
}

const PLAYER_COUNT: usize = 2;

//@thoughts: Should something that takes a mut ref be part of the impl block of
//           that object?  For example should the add_player & add_action
//           methods bellow be member functions as they directly manipulate
//           the DataSource?
impl DataSource {
    pub fn add_game(&mut self) -> Uuid {
        let dealer_id = Uuid::new_v4();
        self.decks.insert(dealer_id, new_deck());
        self.hands.push(Hand {
            id: dealer_id,
            player: dealer_id,
            dealer: dealer_id,
        });
        dealer_id
    }

    //@todo: This needs to return something I guess to indicate success or failure.
    pub fn set_deck(&mut self, game_id: Uuid, deck: Deck) {
        self.decks
            .entry(game_id)
            .and_modify(|current| *current = deck);
    }

    //@todo: this is a little awkward.  We should potentially have a second function to
    // create a hand which returns the hand_id else how does the client know how to add
    // an action?
    pub fn add_player(&mut self, dealer_id: Uuid) -> Uuid {
        let player_id = Uuid::new_v4();
        self.hands.push(Hand {
            id: player_id,
            player: player_id,
            dealer: dealer_id,
        });

        player_id
    }

    //@todo: I think this should this return a uuid; reasons 2 fold, we probably
    //       should have a means to identify the action, and we dont want methods
    //       with no return type.
    pub fn add_action(&mut self, hand_id: Uuid, action: Action) {
        match action {
            Action::Hit => println!("Adding Hit Action"),
            Action::Hold => println!("Adding Hold Action"),
        };
        self.actions.push((hand_id, action));
    }

    pub fn start_game(&mut self, game_id: Uuid) {
        // Every hand gets 2 card
        let mut allocations = Vec::new();
        allocations.extend(allocate_cards(&self.hands, &self.allocations, game_id));
        allocations.extend(allocate_cards(&self.hands, &self.allocations, game_id));

        // Grab the list of the hands that have been updated (this should be all the hands in
        // this game)
        let updated_hands = allocations
            .iter()
            .filter_map(|ca| self.hands.iter().find(|&h| h.id == ca.hand))
            .cloned()
            .collect::<Vec<_>>();

        // Combine the allocations into the master allocation list
        self.allocations.extend(allocations);

        // We now need to check the hand states incase anything interesting has
        // resolved from that.
        let resulting_states = process_hand_states(&updated_hands, &self.allocations, &self.decks);

        // Merge any hand_states into the master state list
        self.hand_states.extend(resulting_states);
    }

    pub fn process_hit_actions(&mut self) {
        let allocations = process_hit_actions(&self.actions, &self.hands, &self.allocations);

        // Check for updates to the hand states.
        let updated_hands = allocations
            .iter()
            .filter_map(|ca| self.hands.iter().find(|&h| h.id == ca.hand))
            .cloned()
            .collect::<Vec<_>>();

        // Merge allocations into the master list.
        self.allocations.extend(allocations);

        // Check if any of the new hands have busted or hit blackjack.
        let resulting_states = process_hand_states(&updated_hands, &self.allocations, &self.decks);
        //todo!("need to add a step here to iterate hand states to check for children that need to be added");

        // Merge into the master state list
        self.hand_states.extend(resulting_states);
    }

    pub fn process_hold_actions(&mut self) {
        let hold_states =
            process_hold_actions(&self.hands, &self.actions, &self.allocations, &self.decks);

        // Merge these into the master state list
        self.hand_states.extend(hold_states);
    }

    pub fn get_next_hand(&self, mut current_hand_idx: usize, turn_order: &[Uuid]) -> usize {
        let mut next_hand_idx = current_hand_idx;
        current_hand_idx = loop {
            next_hand_idx = (next_hand_idx + 1) % PLAYER_COUNT; // Because there are 2 players
            println!("Checking active state of {}", next_hand_idx);
            let hand_id = turn_order[next_hand_idx];
            let is_hand_active = is_hand_active(hand_id, &self.hand_states);
            if is_hand_active {
                println!("Returning {}", next_hand_idx);
                break next_hand_idx;
            }
            if next_hand_idx == current_hand_idx {
                //@note:
                //  We've looped all the hands, none were active which indicates that
                //  the game is over.
                println!("We've iterated all the hands, returning {}", next_hand_idx);
                break next_hand_idx;
            }
        };
        current_hand_idx
    }
}

pub fn allocate_cards(
    hands: &[Hand],
    allocations: &[CardAllocation],
    game_id: Uuid,
) -> Vec<CardAllocation> {
    // Find the current card index into the deck
    let card_idx = allocations.iter().filter(|a| a.dealer == game_id).count();

    // Every hard in the game gets allocated a card
    hands
        .iter()
        .filter(|hand| hand.dealer == game_id)
        .enumerate()
        .map(|(idx, hand)| {
            println!("Adding card allocation: {},{}", hand.id, card_idx);
            CardAllocation {
                card_idx: card_idx + idx,
                dealer: game_id,
                hand: hand.id,
            }
        })
        .collect::<Vec<_>>()
}

pub enum ActionResolutionError {
    MissingHand,
    MissingDeck,
}

// Currently this function has side-effects, I wonder if there is a reasonable
// way to indicate this in rust via naming or something?
//
//@note: Making the assumpting here that there is not going to be any actions
//       in this list that are going to end up hitting the same deck and therefore
//       invalidating the number deck index calculated from the allocations list.
pub fn process_hit_actions(
    actions: &[HandAction],
    hands: &[Hand],
    allocations: &[CardAllocation],
) -> Vec<CardAllocation> {
    println!("Processing Hit Actions");
    actions
        .iter()
        .filter(|(_, action)| matches!(action, Action::Hit))
        .filter_map(|(hand_id, _)| hands.iter().find(|hand| hand.id == *hand_id))
        .map(|hand| {
            let card_idx = allocations
                .iter()
                .filter(|a| a.dealer == hand.dealer)
                .count();
            println!("Adding card allocation: {}", card_idx);
            CardAllocation {
                card_idx,
                dealer: hand.dealer,
                hand: hand.id,
            }
        })
        .collect::<Vec<_>>()
}

pub fn process_hand_states(
    hands: &[Hand],
    card_allocations: &[CardAllocation],
    decks: &HashMap<Uuid, Deck>,
) -> Vec<HandState> {
    let mut hand_states = Vec::new();
    for h in hands {
        let deck = decks.get(&h.dealer).expect("Unable to find deck for table");
        //@note: its probably faster to just build the hand values by iterating this once and building
        //  it as we go foldish style and then map that into a hand_state rather than iterate all the
        //  allocations for each hand like this.
        let cards = card_allocations
            .iter()
            .filter(|a| a.hand == h.id)
            .map(|a| &deck[a.card_idx])
            .collect::<Vec<_>>();

        //@note: its probably better to just not add the actives here rather than strip them out later.
        let hand_value = hand_value(&cards);
        let state = match hand_value {
            0..=20 => State::Active,
            21 => State::BlackJack,
            _ => State::Bust(hand_value),
        };
        hand_states.push((h.id, h.dealer, state));
    }
    // and strip out all the Active's because we dont want to report those.
    hand_states
        .into_iter()
        .filter(|(_, _, hs)| !matches!(hs, State::Active))
        .collect()
}

pub fn process_hold_actions(
    hands: &[Hand],
    actions: &[HandAction],
    allocations: &[CardAllocation],
    decks: &HashMap<Uuid, Deck>,
) -> Vec<HandState> {
    actions
        .iter()
        .filter(|(_, action)| matches!(action, Action::Hold))
        .map(|(hand, _)| {
            // Grab the deck for the hand.
            let dealer_id = get_dealer(*hand, hands);
            let deck = decks
                .get(&dealer_id)
                .expect("Unable to find deck for table");

            // Calculate the hand value
            let cards = allocations
                .iter()
                .filter(|a| a.hand == *hand)
                .map(|a| &deck[a.card_idx])
                .collect::<Vec<_>>();
            let value = hand_value(&cards);
            //@todo: I dont know if I need to check if the hand is blackJack or Bust or anything
            //here.

            (*hand, dealer_id, State::Holding(value))
        })
        .collect::<Vec<_>>()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Won(u8),
    Lost(u8),
}

pub type HandOutcome = (Uuid, Outcome);

// Iterate all of the HandStates in hand_state, for any HandState for which there is a corosponding
// dealer HandState determine the HandOutcome and return it.
pub fn resolve_outcomes(hand_values: &[HandState], outcomes: &[HandOutcome]) -> Vec<HandOutcome> {
    hand_values
        .iter()
        // Check if this particular hand already exists within the outcomes list
        .filter(|h| !outcomes.iter().any(|o| o.0 == h.0))
        // Grab the dealer value and if it exist return (hand, dealer) pair,
        // filter out this hand if the dealer state does not exist.
        .filter_map(|h| hand_values.iter().find(|hv| hv.0 == h.1).map(|d| (h, d)))
        // And finally lets determine the outcome.
        .map(|(h, d)| {
            let state = match d.2 {
                State::BlackJack => Outcome::Lost(0),
                State::Bust(_) => Outcome::Won(22),
                State::Holding(dealer_value) => {
                    match h.2 {
                        State::BlackJack => Outcome::Won(21),
                        State::Bust(v) => Outcome::Lost(v),
                        State::Holding(v) => {
                            if v > dealer_value {
                                Outcome::Won(v)
                            } else {
                                Outcome::Lost(v)
                            }
                        }
                        _ => {
                            unreachable!("Have reached State::Active for a hand while resolving hand outcomes")
                        }
                    }
                },
               _ => unreachable!("The dealer's hand is still active while attempting to resolve the hand outcomes") 
            };
            (h.0, state)
        })
        .collect::<_>()
}

pub fn is_game_complete(dealer: Uuid, hands: &[Hand], hand_states: &[HandState]) -> bool {
    // A game is complete if all of the hands associated with it have HandState's.
    let hand_count = hands.iter().filter(|h| dealer == h.dealer).count();
    let state_count = hand_states.iter().filter(|hs| dealer == hs.1).count();
    hand_count == state_count
}

// @todo:  I guess we need to keep this "clone" but I dont like it.
pub fn get_hand_outcome(hand_id: Uuid, outcomes: &[HandOutcome]) -> Option<Outcome> {
    outcomes
        .iter()
        .find(|o| o.0 == hand_id)
        .map(|o| o.1.clone())
}

// turn sequence; the order in which players take turns (with the dealer going last)
// action validation; an action is only balid if its that players turn to go.
// Rules engine?
