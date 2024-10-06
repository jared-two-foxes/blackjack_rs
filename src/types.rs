use std::fmt;
use uuid::Uuid;

#[derive(Clone)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl fmt::Debug for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hearts => write!(f, "H"),
            Self::Diamonds => write!(f, "D"),
            Self::Clubs => write!(f, "C"),
            Self::Spades => write!(f, "S"),
        }
    }
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone)]
pub enum CardValue {
    Ace,
    King,
    Queen,
    Jack,
    Value(u8),
}

impl fmt::Debug for CardValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ace => write!(f, "A"),
            Self::King => write!(f, "K"),
            Self::Queen => write!(f, "Q"),
            Self::Jack => write!(f, "J"),
            Self::Value(value) => write!(f, "{}", value),
        }
    }
}

impl fmt::Display for CardValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone)]
pub struct Card {
    pub suit: Suit,
    pub value: CardValue,
}

impl Card {
    pub fn new(suit: Suit, value: CardValue) -> Card {
        Card { suit, value }
    }
}

impl fmt::Debug for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Card [{}{}]", self.suit, self.value)
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
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
#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    Won(u8),
    Lost(u8),
}

pub type HandOutcome = (Uuid, Outcome);
