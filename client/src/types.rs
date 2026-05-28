use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableState {
    pub game_state: String,
    pub seconds_remaining: Option<u64>,
    pub hands: Vec<HandInfo>,
    pub outcomes: Vec<(Uuid, String)>,
    pub bets: Vec<Bet>,
    pub active_hands: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandInfo {
    pub hand: Hand,
    pub cards: Vec<Card>,
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hand {
    pub id: Uuid,
    pub player: Uuid,
    pub dealer: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub suit: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bet {
    pub hand_id: Uuid,
    pub player_id: Uuid,
    pub dealer_id: Uuid,
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub id: Uuid,
    pub state: String,
    pub player_count: usize,
    pub seconds_remaining: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinResponse {
    pub hand_id: Option<Uuid>,
    pub already_seated: bool,
    pub rejected: bool,
    pub reason: Option<String>,
}

/// Actions the player can submit. Serialises as a plain string (e.g. `"Hit"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ActionMsg {
    Hit,
    Hold,
    DoubleDown,
    Split,
    Surrender,
}

impl std::fmt::Display for ActionMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ActionMsg::Hit => "Hit",
            ActionMsg::Hold => "Hold",
            ActionMsg::DoubleDown => "DoubleDown",
            ActionMsg::Split => "Split",
            ActionMsg::Surrender => "Surrender",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_msg_serialises_as_plain_string() {
        assert_eq!(serde_json::to_string(&ActionMsg::Hit).unwrap(), r#""Hit""#);
        assert_eq!(
            serde_json::to_string(&ActionMsg::Hold).unwrap(),
            r#""Hold""#
        );
        assert_eq!(
            serde_json::to_string(&ActionMsg::DoubleDown).unwrap(),
            r#""DoubleDown""#
        );
        assert_eq!(
            serde_json::to_string(&ActionMsg::Split).unwrap(),
            r#""Split""#
        );
        assert_eq!(
            serde_json::to_string(&ActionMsg::Surrender).unwrap(),
            r#""Surrender""#
        );
    }

    #[test]
    fn action_msg_round_trip() {
        for action in [
            ActionMsg::Hit,
            ActionMsg::Hold,
            ActionMsg::DoubleDown,
            ActionMsg::Split,
            ActionMsg::Surrender,
        ] {
            let json = serde_json::to_string(&action).unwrap();
            let restored: ActionMsg = serde_json::from_str(&json).unwrap();
            assert_eq!(action, restored);
        }
    }

    #[test]
    fn card_value_numeric_deserialises() {
        let json = r#"{"suit":"Hearts","value":{"Value":9}}"#;
        let card: Card = serde_json::from_str(json).unwrap();
        assert_eq!(card.suit, "Hearts");
        // value should be an object {"Value":9}
        assert!(card.value.is_object());
    }

    #[test]
    fn card_value_face_deserialises() {
        let json = r#"{"suit":"Spades","value":"Ace"}"#;
        let card: Card = serde_json::from_str(json).unwrap();
        assert_eq!(card.suit, "Spades");
        assert_eq!(card.value.as_str().unwrap(), "Ace");
    }
}
