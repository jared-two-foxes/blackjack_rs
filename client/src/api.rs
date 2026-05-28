use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::types::{ActionMsg, JoinResponse, TableInfo, TableState};

pub async fn create_player(base: &str) -> Result<(Uuid, u32)> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/player", base))
        .send()
        .await?
        .error_for_status()?;
    let body: serde_json::Value = resp.json().await?;
    let player_id = body["player_id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing player_id"))?
        .parse::<Uuid>()?;
    let balance = body["balance"]
        .as_u64()
        .ok_or_else(|| anyhow!("missing balance"))? as u32;
    Ok((player_id, balance))
}

pub async fn get_player(base: &str, id: Uuid) -> Result<(Uuid, u32)> {
    let resp = reqwest::get(format!("{}/player/{}", base, id))
        .await?
        .error_for_status()?;
    let body: serde_json::Value = resp.json().await?;
    let player_id = body["player_id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing player_id"))?
        .parse::<Uuid>()?;
    let balance = body["balance"]
        .as_u64()
        .ok_or_else(|| anyhow!("missing balance"))? as u32;
    Ok((player_id, balance))
}

pub async fn get_tables(base: &str) -> Result<Vec<TableInfo>> {
    let resp = reqwest::get(format!("{}/tables", base))
        .await?
        .error_for_status()?;
    let tables: Vec<TableInfo> = resp.json().await?;
    Ok(tables)
}

pub async fn join_table(base: &str, table_id: Uuid, player_id: Uuid) -> Result<JoinResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/table/{}/join", base, table_id))
        .json(&serde_json::json!({ "player_id": player_id }))
        .send()
        .await?
        .error_for_status()?;
    let join_resp: JoinResponse = resp.json().await?;
    Ok(join_resp)
}

pub async fn leave_table(base: &str, table_id: Uuid, player_id: Uuid) -> Result<()> {
    let client = reqwest::Client::new();
    client
        .post(format!("{}/table/{}/leave", base, table_id))
        .json(&serde_json::json!({ "player_id": player_id }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn place_bet(base: &str, table_id: Uuid, player_id: Uuid, amount: u32) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/table/{}/bet", base, table_id))
        .json(&serde_json::json!({ "player_id": player_id, "amount": amount }))
        .send()
        .await?
        .error_for_status()?;
    let body: serde_json::Value = resp.json().await?;
    if let Some(err) = body["error"].as_str() {
        if !err.is_empty() {
            return Err(anyhow!("bet error: {}", err));
        }
    }
    Ok(())
}

pub async fn get_table(base: &str, table_id: Uuid) -> Result<TableState> {
    let resp = reqwest::get(format!("{}/table/{}", base, table_id))
        .await?
        .error_for_status()?;
    let state: TableState = resp.json().await?;
    Ok(state)
}

pub async fn submit_action(base: &str, hand_id: Uuid, action: ActionMsg) -> Result<()> {
    let client = reqwest::Client::new();
    client
        .post(format!("{}/action", base))
        .json(&serde_json::json!({ "hand_id": hand_id, "action": action }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
