use log::info;
use std::sync::mpsc;
use uuid::Uuid;

use crate::data_source::DataSource;
use crate::types::*;
use crate::utils::*;

pub enum Resource {
    Game,
    Player,
    HandAction,
}

// @todo: This needs tho have a header that includes the game_id and potentially the hand or
// player?
pub enum Response {
    StatusOk,
    AddResource(Resource, Uuid),
    TableList(Vec<Uuid>), //< I think maybe this should be a json object or something?
    Hand(Uuid),
    HandValue(u8),
    HandOutcome(Option<Outcome>),
    Failed,
}

#[derive(Debug)]
pub enum Message {
    AddPlayer(Uuid /*game_id*/),
    AddHandAction(Uuid /*hand_id*/, Action),
    GetTableList,
    //@todo: This shouldnt be a client message, needs to get handled in the simulation step.
    //StartGame(Uuid /*game_id*/),
    GetCurrentHand(Uuid /*game_id*/),
    GetHandValue(Uuid /*hand_id*/),
    GetHandOutcome(Uuid /*hand_id*/),
}

pub struct MessagePacket {
    pub message: Message,
    pub response_tx: mpsc::Sender<Response>,
}

// The backend loop is essentially a message router, responding and processing incoming
// client messages and determining the appropriate response to return to the client
//
//@todo:
//      I think that we maybe need to move out anything to do with advancing the
//      simulation from here instead I guess "mark" the update required for the simulation
//      step?
pub fn process(rx: &mpsc::Receiver<MessagePacket>, ds: &mut DataSource) {
    if let Ok(message_packet) = rx.try_recv() {
        let response = match message_packet.message {
            Message::AddPlayer(game_id) => {
                info!("server: AddPlayer");
                let player_id = ds.add_player(game_id);
                Response::AddResource(Resource::Player, player_id)
            }
            Message::AddHandAction(_hand_id, _action) => {
                info!("server: AddHandAction");
                //@todo: This should go into some kinda temp buffer that then gets
                //  processed by the simulation step.  I guess that means that
                //  there shouldnt be a response here, maybe just an ack?
                //ds.add_action(hand_id, action);
                //let new_uuid = Uuid::new_v4();
                //Response::AddResource(Resource::HandAction, new_uuid)
                Response::Failed
            }
            Message::GetTableList => {
                //@todo: Should this only respond with open tables?
                let tables: Vec<Uuid> = ds.decks.keys().cloned().collect();
                Response::TableList(tables)
            }
            //@todo: Starting the game should not be a client side message
            /*Message::StartGame(game_id) => {
                info!("server: StartGame");
                ds.start_game(game_id);
                Response::StatusOk
            }*/
            Message::GetCurrentHand(game_id) => {
                info!("server: GetCurrentHand");
                get_active_hand(game_id, &ds.active_hands, &ds.hands)
                    .map_or(Response::Failed, Response::Hand)
            }
            Message::GetHandValue(hand_id) => {
                info!("server: GetHandValue");
                let hand_value = get_hand_value(hand_id, &ds.hands, &ds.allocations, &ds.decks);
                Response::HandValue(hand_value)
            }
            Message::GetHandOutcome(hand_id) => {
                info!("server: GetHandOutcome");
                let hand_outcome = get_hand_outcome(hand_id, &ds.outcomes);
                Response::HandOutcome(hand_outcome)
            }
        };
        message_packet.response_tx.send(response).unwrap();
    }
}
