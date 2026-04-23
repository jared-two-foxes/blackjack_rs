use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use blackjack::app_and_state;
use serde_json::json;
use tower::util::ServiceExt; // for oneshot
use uuid::Uuid;

#[tokio::test]
async fn test_player_join_and_leave_table() {
    let app = app_and_state();

    // Create a new table by starting a game (simulate by joining as dealer)
    let table_id = Uuid::new_v4();
    let player_id = Uuid::new_v4();

    // Join the table
    let join_req = json!({ "player_id": player_id });
    let req = Request::post(format!("/table/{}/join", table_id))
        .header("content-type", "application/json")
        .body(Body::from(join_req.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let join_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(join_resp.get("hand_id").unwrap().as_str().is_some());
    assert_eq!(join_resp.get("already_seated").unwrap(), false);

    // Try joining again (should not duplicate)
    let req = Request::post(format!("/table/{}/join", table_id))
        .header("content-type", "application/json")
        .body(Body::from(join_req.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let join_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(join_resp.get("hand_id").unwrap().is_null());
    assert_eq!(join_resp.get("already_seated").unwrap(), true);

    // Leave the table
    let leave_req = json!({ "player_id": player_id });
    let req = Request::post(format!("/table/{}/leave", table_id))
        .header("content-type", "application/json")
        .body(Body::from(leave_req.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let leave_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(leave_resp.get("left").unwrap(), true);
    assert_eq!(leave_resp.get("hand_id").unwrap().as_str().is_some(), true);

    // Try leaving again (should be a no-op)
    let req = Request::post(format!("/table/{}/leave", table_id))
        .header("content-type", "application/json")
        .body(Body::from(leave_req.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let leave_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(leave_resp.get("left").unwrap(), false);
    assert!(leave_resp.get("hand_id").unwrap().is_null());
}
