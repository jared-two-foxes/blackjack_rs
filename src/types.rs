use uuid::Uuid;

#[derive(Clone)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Clone)]
pub enum Value {
    Ace,
    King,
    Queen,
    Jack,
    Value(u8),
}

pub type Card = (Suit, Value);

pub type Deck = Vec<Card>;

#[derive(Clone)]
pub struct Hand {
    pub id: Uuid,
    pub player: Uuid,
    pub dealer: Uuid,
}

pub struct CardAllocation {
    pub hand: Uuid,
    pub dealer: Uuid, //< this is also dealer's uuid since that is how we identify specific decks.
    pub card_idx: usize,
}

#[derive(Debug)]
pub struct Sequence {
    pub game_id: Uuid,
    pub hand_id: Uuid,
}

// @todo: I've seen this Hold referenced as "Stand" which I guess makes more sense?
#[derive(Debug)]
pub enum Action {
    Hit,
    Hold,
}

#[derive(Debug)]
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

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Won(u8),
    Lost(u8),
}

pub type HandOutcome = (Uuid, Outcome);
