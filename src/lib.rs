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

pub struct Hand {
    pub id: Uuid,
    player: Uuid,
    pub game: Uuid,
    dealer: bool,
}

fn hand_value(cards: &Vec<&Card>) -> u8 {
    let mut ace_count = 0;
    let mut value = cards
        .iter()
        .map(|c| match c.1 {
            Value::Value(v) => v,
            Value::Ace => {
                ace_count = ace_count + 1;
                11
            }
            _ => 10,
        })
        .sum::<u8>();
    for _ in 0..ace_count {
        if value > 21 {
            value = value - 10;
        }
    }
    value
}

fn hand_bust(cards: &Vec<&Card>) -> bool {
    hand_value(cards) > 21
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
    pub card_idx: usize,
}

//@todo: i think its a good idea to split this out since the outcome of these actions is different which is making it hard to build out the functions to process as the return types get hard
pub enum Action {
    Hit,
    Hold,
}

//< @todo:  We should store the hand value here.
pub enum State {
    Active,
    Holding(u8),
    Bust,
}

// Pair mapping hand to its current state.
pub type HandState = (Uuid, State);

//pair mapping hand to an action
pub type HandAction = (Uuid, Action);

#[derive(Default)]
pub struct DataSource {
    pub hands: Vec<Hand>,
    pub decks: HashMap<Uuid, Deck>, // map of game_id to Deck for a given game
    pub allocations: Vec<CardAllocation>,
    pub hand_states: Vec<HandState>,
    pub actions: Vec<HandAction>,
}

//@thoughts: Should something that takes a mut ref be part of the impl block of
//           that object?  For example should the add_player & add_action
//           methods bellow be member functions as they directly manipulate
//           the DataSource?

pub fn add_game(ds: &mut DataSource) -> Uuid {
    let game_id = Uuid::default();
    ds.decks.insert(game_id, new_deck());
    ds.hands.push(Hand {
        id: Uuid::default(),
        player: Uuid::default(),
        game: game_id,
        dealer: true,
    });
    game_id
}

pub fn add_player(ds: &mut DataSource, game_id: Uuid) -> Uuid {
    let player_id = Uuid::default();
    ds.hands.push(Hand {
        id: Uuid::default(),
        player: player_id,
        game: game_id,
        dealer: false,
    });

    player_id
}

//@todo: I think this should this return a uuid; reasons 2 fold, we probably
//       should have a means to identify the action, and we dont want methods
//       with no return type.
pub fn add_action(ds: &mut DataSource, hand_id: Uuid, action: Action) {
    ds.actions.push((hand_id, action));
}

//
// Systems/Controllers
//

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
    actions: &Vec<HandAction>,
    allocations: &[CardAllocation],
) -> Vec<CardAllocation> {
    actions.iter().filter(|(_,action) matches!(Action::Hit, action)).map(|(hand,_)|
    {
                let card_idx = allocations.iter().filter(|a| a.hand == *hand).count();
                new_allocations.push(CardAllocation {
                    card_idx,
                    hand: *hand,
                });
            }
    }).collect::<Vec<_>>()
}

pub fn process_hand_states(
    hands: &[Hand],
    card_allocations: &[CardAllocation],
    decks: &HashMap<Uuid, Deck>,
) -> Vec<HandState> {
    let mut hand_states = Vec::new();
    for h in hands {
        let deck = decks.get(&h.game).expect("Unable to find deck for table");
        //@note: its probably faster to just build the hand values by iterating this once and building it as we go foldish style and then map that into a hand_state rather than iterate all the allocations for each hand like this.
        let cards = card_allocations
            .iter()
            .filter(|a| a.hand == h.id)
            .map(|a| &deck[a.card_idx])
            .collect::<Vec<_>>();

        //@note: its probably better to just not add the actives here rather than strip them out later.
        let state = match  hand_value(&cards) {
            0..=20 => State::Active,
            21 => State::BlackJack,
            _ => State::Bust,
        };
        hand_states.push((h.id, state));
    }
    
    // and strip out all the Active's because we dont want to report those.
    
    hand_states.iter().filter(|hs| not matches!(hs, State::Active)).collect()
}

pub fn process_hold_actions(
    actions: &Vec<HandAction>,
    allocations: &[CardAllocation],
    decks: &HashMap<Uuid, Deck>
) -> Vec<HandState> {
    actions.iter().filter(|(_,action) matches!(Action::Hold, action)).map(|(hand,_)| {
    let deck = decks.get(&hand.game).expect("Unable to find deck for table");
    let cards = card_allocations
            .iter()
            .filter(|a| a.hand == h.id)
            .map(|a| &deck[a.card_idx])
            .collect::<Vec<_>>();
    HandState(hand, State::Holding(hand_value(cards)) 
    }).collect::<Vec<_>>()
}

/*pub fn resolve_table_state(
    decks: &HashMap<Uuid, Deck>,
    hands: &[Hand],
    allocations: &[CardAllocation],
) -> Vec<HandState> {
    let mut hand_states = Vec::new();

    //@note: We have one deck per table so iterating over the keys is a way to
    //       get all of the table identifiers but we are also going to want the
    //       decks so we're just going to iterate over the decks list.
    decks.iter().for_each(|(&game_id, deck)| {
        // Retrieve all of the hands being played on this table.
        let game_hands = hands
            .iter()
            .filter(|h| h.game == game_id)
            .collect::<Vec<_>>();

        // Grab all the cards for the hands at the table.
        let hand_cards = game_hands
            .iter()
            .map(|h| {
                (
                    h.id,
                    allocations
                        .iter()
                        .filter(|a| a.hand == h.id)
                        .map(|a| &deck[a.card_idx])
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        let hand_values = hand_cards
            .iter()
            .map(|(id, cards)| {
                let value = hand_value(cards);
                let state = if value > 21 {
                    State::Bust
                } else {
                    State::Active
                };
                (id, value, state)
            })
            .collect::<Vec<_>>();
    });
    hand_states
}*/

pub enum Outcome {
    Won,
    Lost,
}

pub type HandOutcome = (Uuid, Outcome);

//
pub fn resolve_outcomes(hands: &Vec<Hand>, hand_values: &Vec<(Uuid, u8)>) -> Vec<HandOutcome> {
    let outcomes = Vec::new();
    
    //@todo: partition list based on completed tables? Group by table? Extract dealers? 

    // we could potentially order the hands list to ensure that all the dealers are listed first.  also might make sense to have the dealers uuid on the hand rather than a "table", a table data struct doesnt really do much where as we can use dealer like a "parent" and then order that way.
    
    // i do like this idea but im not quite sure how to get it to work.  i think there needs to be a step in response to adding a new hand_state that walks the list looking for children of a given parent.
    
    // identify the dealer on the table.
    let dealer = hands
        .iter()
        .find(|h| h.dealer)
        .expect("Unable to find dealer");

    // Extract the dealers hand from the list
    let dealer_idx = hand_values
        .iter()
        .position(|(&id, _)| id == dealer.id)
        .expect("Unable to find the dealers hand state");
    let (_, dealer_hand_value) = hand_values.remove(dealer_idx);

    // Check if the dealer has bust
    if dealer_hand_value > 21 {
        unimplemented!(); //@todo: All non-busted hands win.
    } else if dealer_hand_value == 21 { 
        // all hands lose
    } else {  
        // hand_values now only contains the player hands so iterate and
        // compare them all to the dealer.
        hand_values
            .iter()
            .map(|(id, value)| {
                let state = if value > dealer_hand_value {
                    Outcome::Won
                } else {
                    Outcome::Lost
                };
                (id, state)
            })
            .collect::<Vec<_>>()
    }

    outcomes
}

// turn sequence; the order in which players take turns (with the dealer going last)

// action validation; an action is only balid if its that players turn to go.

// Rules engine?
