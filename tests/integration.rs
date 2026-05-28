//! Integration tests for the blackjack Axum server.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use blackjack::app_and_state;
use serde_json::json;
use tower::util::ServiceExt; // for oneshot

/// Submitting an action for a hand that does not exist must return 400.
/// Submitting actions for non-existent hands is always invalid — the server
/// must reject it rather than silently queuing garbage.
#[tokio::test]
async fn test_submit_action_unknown_hand_returns_400() {
    let app = app_and_state();

    let action = json!({
        "hand_id": "00000000-0000-0000-0000-000000000000",
        "action": "Hit"
    });
    let req = Request::post("/action")
        .header("content-type", "application/json")
        .body(Body::from(action.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// GET /table/:id always returns 200 with a JSON body containing a `hands` field,
/// even for a table UUID that has no hands (empty slice expected).
#[tokio::test]
async fn test_get_table_state_returns_200_and_hands_field() {
    let app = app_and_state();

    let req = Request::get("/table/00000000-0000-0000-0000-000000000000")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let state: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        state.get("hands").is_some(),
        "response must contain 'hands' field"
    );
}
