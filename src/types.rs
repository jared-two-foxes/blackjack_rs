use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Clone, Serialize, Deserialize)]
pub struct Hand {
    pub id: Uuid,
    pub player: Uuid,
    pub dealer: Uuid,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CardAllocation {
    pub hand: Uuid,
    pub dealer: Uuid, //< this is also dealer's uuid since that is how we identify specific decks.
    pub card_idx: usize,
}

#[derive(Debug, Clone)]
pub struct Sequence {
    pub game_id: Uuid,
    pub hand_id: Uuid,
}

// @todo: I've seen this Hold referenced as "Stand" which I guess makes more sense?
#[derive(Debug, Clone, Copy)]
pub enum Action {
    Hit,
    Hold,
    DoubleDown,
    Split,
    Surrender,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum State {
    Active,
    Holding(u8),
    Bust(u8),
    BlackJack,
    Surrendered,
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
    Push,
    Surrendered,
}

pub type HandOutcome = (Uuid, Outcome);

#[derive(Debug, Clone, PartialEq)]
pub enum BetError {
    BelowMinimum,
    InsufficientFunds,
    WrongGameState,
    DuplicateBet,
    PlayerNotFound,
}

impl std::fmt::Display for BetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            BetError::PlayerNotFound => "player not found",
            BetError::BelowMinimum => "bet below minimum",
            BetError::InsufficientFunds => "insufficient funds",
            BetError::WrongGameState => "table is not accepting bets",
            BetError::DuplicateBet => "player already has a bet at this table",
        };
        write!(f, "{}", msg)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: Uuid,
    pub balance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bet {
    pub hand_id: Uuid,
    pub player_id: Uuid,
    pub dealer_id: Uuid,
    pub amount: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_push_equality() {
        assert_eq!(Outcome::Push, Outcome::Push);
        assert_ne!(Outcome::Push, Outcome::Won(1));
    }

    #[test]
    fn card_serde_round_trip() {
        let original = Card::new(Suit::Hearts, CardValue::Ace);
        let json = serde_json::to_string(&original).expect("serialize failed");
        let restored: Card = serde_json::from_str(&json).expect("deserialize failed");
        // Verify suit and value match via their Debug representations
        assert_eq!(
            format!("{:?}", original.suit),
            format!("{:?}", restored.suit)
        );
        assert_eq!(
            format!("{:?}", original.value),
            format!("{:?}", restored.value)
        );
    }

    #[test]
    fn player_serde_round_trip() {
        let id = Uuid::new_v4();
        let original = Player { id, balance: 500 };
        let json = serde_json::to_string(&original).expect("serialize failed");
        let restored: Player = serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(original.id, restored.id);
        assert_eq!(original.balance, restored.balance);
    }

    #[test]
    fn bet_serde_round_trip() {
        let original = Bet {
            hand_id: Uuid::new_v4(),
            player_id: Uuid::new_v4(),
            dealer_id: Uuid::new_v4(),
            amount: 100,
        };
        let json = serde_json::to_string(&original).expect("serialize failed");
        let restored: Bet = serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(original.hand_id, restored.hand_id);
        assert_eq!(original.player_id, restored.player_id);
        assert_eq!(original.dealer_id, restored.dealer_id);
        assert_eq!(original.amount, restored.amount);
    }
}
