//! Integration tests for the blackjack Axum server.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use blackjack::app_and_state;
use serde_json::json;
use tower::util::ServiceExt; // for oneshot

#[tokio::test]
async fn test_submit_action_and_get_table_state() {
    let app = app_and_state();

    // Submit an action (replace with actual hand_id and action as needed)
    let action = json!({
        "hand_id": "00000000-0000-0000-0000-000000000000",
        "action": "Hit"
    });
    let req = Request::post("/action")
        .header("content-type", "application/json")
        .body(Body::from(action.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert!(body
        .windows("Action received".len())
        .any(|w| w == b"Action received"));

    // Get table state (replace with actual table_id as needed)
    let req = Request::get("/table/00000000-0000-0000-0000-000000000000")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let state: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(state.get("hands").is_some());
}
