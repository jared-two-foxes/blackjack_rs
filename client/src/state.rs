use uuid::Uuid;

use crate::types::{ActionMsg, Card, TableInfo, TableState};

#[derive(Debug, Clone)]
pub enum Screen {
    Lobby,
    Betting { table_id: Uuid, hand_id: Uuid },
    Game { table_id: Uuid, hand_id: Uuid },
    Result { table_id: Uuid },
}

pub struct App {
    pub screen: Screen,
    pub player_id: Uuid,
    pub balance: u32,
    pub tables: Vec<TableInfo>,
    pub selected_table: usize,
    pub table_state: Option<TableState>,
    pub bet_input: String,
    pub status_msg: Option<String>,
    pub action_staged: Option<ActionMsg>,
}

impl App {
    pub fn new(player_id: Uuid, balance: u32) -> Self {
        App {
            screen: Screen::Lobby,
            player_id,
            balance,
            tables: Vec::new(),
            selected_table: 0,
            table_state: None,
            bet_input: String::new(),
            status_msg: None,
            action_staged: None,
        }
    }
}

/// Returns the eligible actions for the player's current hand.
pub fn available_actions(app: &App) -> Vec<ActionMsg> {
    let hand_id = match &app.screen {
        Screen::Game { hand_id, .. } => *hand_id,
        _ => return vec![],
    };

    let ts = match &app.table_state {
        Some(ts) => ts,
        None => return vec![],
    };

    // Find the player's HandInfo
    let hand_info = match ts.hands.iter().find(|h| h.hand.id == hand_id) {
        Some(h) => h,
        None => return vec![],
    };

    // Hand is active if state is None or "active"
    let is_active = hand_info
        .state
        .as_deref()
        .map(|s| s == "active")
        .unwrap_or(true);

    if !is_active {
        return vec![];
    }

    let card_count = hand_info.cards.len();

    // Find the bet for this hand
    let bet_amount = ts
        .bets
        .iter()
        .find(|b| b.hand_id == hand_id)
        .map(|b| b.amount)
        .unwrap_or(0);

    let mut actions = vec![ActionMsg::Hit, ActionMsg::Hold];

    if card_count == 2 {
        // DoubleDown: active + exactly 2 cards + balance >= bet
        if app.balance >= bet_amount {
            actions.push(ActionMsg::DoubleDown);
        }

        // Split: active + exactly 2 cards + both cards same split-category + balance >= bet
        if app.balance >= bet_amount && cards_are_splittable(&hand_info.cards) {
            actions.push(ActionMsg::Split);
        }

        // Surrender: active + exactly 2 cards
        actions.push(ActionMsg::Surrender);
    }

    actions
}

/// Returns true if two cards have the same split-category value.
/// Face cards (King, Queen, Jack) all count as 10 for split purposes.
fn cards_are_splittable(cards: &[Card]) -> bool {
    if cards.len() != 2 {
        return false;
    }
    split_category(&cards[0]) == split_category(&cards[1])
}

/// Returns a canonical split-category string for a card value.
fn split_category(card: &Card) -> String {
    match &card.value {
        serde_json::Value::String(s) => match s.as_str() {
            "King" | "Queen" | "Jack" => "10".to_string(),
            other => other.to_string(),
        },
        serde_json::Value::Object(map) => {
            if let Some(n) = map.get("Value").and_then(|v| v.as_u64()) {
                n.to_string()
            } else {
                "?".to_string()
            }
        }
        other => other.to_string(),
    }
}

/// Formats a card as e.g. `"A♠"`, `"K♥"`, `"9♦"`, `"2♣"`.
pub fn card_display(card: &Card) -> String {
    let suit_sym = match card.suit.as_str() {
        "Hearts" => "♥",
        "Diamonds" => "♦",
        "Clubs" => "♣",
        "Spades" => "♠",
        other => other,
    };

    let value_str = match &card.value {
        serde_json::Value::String(s) => match s.as_str() {
            "Ace" => "A".to_string(),
            "King" => "K".to_string(),
            "Queen" => "Q".to_string(),
            "Jack" => "J".to_string(),
            other => other.to_string(),
        },
        serde_json::Value::Object(map) => {
            if let Some(n) = map.get("Value").and_then(|v| v.as_u64()) {
                n.to_string()
            } else {
                "?".to_string()
            }
        }
        other => other.to_string(),
    };

    format!("{}{}", value_str, suit_sym)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Bet, Hand, HandInfo, TableState};

    fn make_card(suit: &str, value: serde_json::Value) -> Card {
        Card {
            suit: suit.to_string(),
            value,
        }
    }

    fn face_card(suit: &str, face: &str) -> Card {
        make_card(suit, serde_json::Value::String(face.to_string()))
    }

    fn num_card(suit: &str, n: u64) -> Card {
        make_card(suit, serde_json::json!({"Value": n}))
    }

    #[test]
    fn card_display_ace_spades() {
        let c = face_card("Spades", "Ace");
        assert_eq!(card_display(&c), "A♠");
    }

    #[test]
    fn card_display_king_hearts() {
        let c = face_card("Hearts", "King");
        assert_eq!(card_display(&c), "K♥");
    }

    #[test]
    fn card_display_queen_diamonds() {
        let c = face_card("Diamonds", "Queen");
        assert_eq!(card_display(&c), "Q♦");
    }

    #[test]
    fn card_display_jack_clubs() {
        let c = face_card("Clubs", "Jack");
        assert_eq!(card_display(&c), "J♣");
    }

    #[test]
    fn card_display_nine_diamonds() {
        let c = num_card("Diamonds", 9);
        assert_eq!(card_display(&c), "9♦");
    }

    #[test]
    fn card_display_two_clubs() {
        let c = num_card("Clubs", 2);
        assert_eq!(card_display(&c), "2♣");
    }

    fn make_app_with_hand(
        cards: Vec<Card>,
        hand_state: Option<&str>,
        balance: u32,
        bet_amount: u32,
    ) -> App {
        let player_id = Uuid::new_v4();
        let dealer_id = Uuid::new_v4();
        let hand_id = Uuid::new_v4();

        let hand = Hand {
            id: hand_id,
            player: player_id,
            dealer: dealer_id,
        };

        let hand_info = HandInfo {
            hand: hand.clone(),
            cards,
            state: hand_state.map(|s| s.to_string()),
        };

        let bet = Bet {
            hand_id,
            player_id,
            dealer_id,
            amount: bet_amount,
        };

        let ts = TableState {
            game_state: "Active".to_string(),
            seconds_remaining: None,
            hands: vec![hand_info],
            outcomes: vec![],
            bets: vec![bet],
            active_hands: vec![hand_id],
        };

        let table_id = Uuid::new_v4();
        let mut app = App::new(player_id, balance);
        app.screen = Screen::Game { table_id, hand_id };
        app.table_state = Some(ts);
        app
    }

    #[test]
    fn available_actions_active_two_cards_sufficient_balance() {
        let cards = vec![face_card("Hearts", "King"), face_card("Spades", "King")];
        let app = make_app_with_hand(cards, None, 200, 100);
        let actions = available_actions(&app);
        assert!(actions.contains(&ActionMsg::Hit));
        assert!(actions.contains(&ActionMsg::Hold));
        assert!(actions.contains(&ActionMsg::DoubleDown));
        assert!(actions.contains(&ActionMsg::Split));
        assert!(actions.contains(&ActionMsg::Surrender));
    }

    #[test]
    fn available_actions_active_two_cards_insufficient_balance() {
        let cards = vec![face_card("Hearts", "King"), face_card("Spades", "Queen")];
        // balance < bet_amount → no DoubleDown, no Split
        let app = make_app_with_hand(cards, None, 50, 100);
        let actions = available_actions(&app);
        assert!(actions.contains(&ActionMsg::Hit));
        assert!(actions.contains(&ActionMsg::Hold));
        assert!(!actions.contains(&ActionMsg::DoubleDown));
        assert!(!actions.contains(&ActionMsg::Split));
        assert!(actions.contains(&ActionMsg::Surrender));
    }

    #[test]
    fn available_actions_active_three_cards() {
        let cards = vec![
            face_card("Hearts", "King"),
            face_card("Spades", "Queen"),
            num_card("Clubs", 2),
        ];
        let app = make_app_with_hand(cards, None, 200, 100);
        let actions = available_actions(&app);
        assert!(actions.contains(&ActionMsg::Hit));
        assert!(actions.contains(&ActionMsg::Hold));
        assert!(!actions.contains(&ActionMsg::DoubleDown));
        assert!(!actions.contains(&ActionMsg::Split));
        assert!(!actions.contains(&ActionMsg::Surrender));
    }

    #[test]
    fn available_actions_inactive_hand() {
        let cards = vec![face_card("Hearts", "King"), face_card("Spades", "Queen")];
        let app = make_app_with_hand(cards, Some("bust"), 200, 100);
        let actions = available_actions(&app);
        assert!(actions.is_empty());
    }

    #[test]
    fn available_actions_not_in_game_screen() {
        let player_id = Uuid::new_v4();
        let app = App::new(player_id, 1000);
        // screen is Lobby
        let actions = available_actions(&app);
        assert!(actions.is_empty());
    }

    #[test]
    fn split_requires_same_category() {
        // King and Queen both map to "10" → splittable
        let cards = vec![face_card("Hearts", "King"), face_card("Spades", "Queen")];
        let app = make_app_with_hand(cards, None, 200, 100);
        let actions = available_actions(&app);
        assert!(actions.contains(&ActionMsg::Split));

        // King and Ace → not splittable
        let cards2 = vec![face_card("Hearts", "King"), face_card("Spades", "Ace")];
        let app2 = make_app_with_hand(cards2, None, 200, 100);
        let actions2 = available_actions(&app2);
        assert!(!actions2.contains(&ActionMsg::Split));
    }

    #[test]
    fn split_numeric_same_value() {
        let cards = vec![num_card("Hearts", 8), num_card("Spades", 8)];
        let app = make_app_with_hand(cards, None, 200, 100);
        let actions = available_actions(&app);
        assert!(actions.contains(&ActionMsg::Split));
    }
}
