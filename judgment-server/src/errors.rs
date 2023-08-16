use axum::{http::StatusCode, response::IntoResponse, Json};
use judgment::InvalidTransition;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, thiserror::Error, Serialize)]
#[error("room is full, capacity: {max_players}")]
pub struct RoomFull {
    pub max_players: u8,
}

#[derive(Debug, thiserror::Error)]
#[error("cannot play cards or predict scores yet")]
pub struct TooEarly;

#[derive(Debug, thiserror::Error, Serialize)]
#[error("not a valid player Id")]
pub struct InvalidPlayerId;

#[derive(Debug, Serialize)]
pub struct InvalidToken;

impl IntoResponse for InvalidToken {
    fn into_response(self) -> axum::response::Response {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

#[derive(Debug, Serialize)]
pub struct ServerFull;

impl IntoResponse for ServerFull {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::CONFLICT,
            Json(json!({"error": "no space left in server for another game"})),
        )
            .into_response()
    }
}

#[derive(Debug, thiserror::Error, Serialize)]
#[error("no such room exists")]
pub struct InvalidRoomId;

impl IntoResponse for InvalidRoomId {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::NOT_FOUND,
            Json(json!({"error": self.to_string()})),
        )
            .into_response()
    }
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum RoomJoinError {
    #[error(transparent)]
    InvalidRoomId(#[from] InvalidRoomId),
    #[error(transparent)]
    RoomFull(#[from] RoomFull),
}

impl IntoResponse for RoomJoinError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": self.to_string()})),
        )
            .into_response()
    }
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum PlayError {
    #[error(transparent)]
    InvalidRoomId(#[from] InvalidRoomId),
    #[error(transparent)]
    InvalidTransition(#[from] InvalidTransition),
}

impl IntoResponse for PlayError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": self.to_string()})),
        )
            .into_response()
    }
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum ResourceDoesNotExist {
    #[error(transparent)]
    Room(#[from] InvalidRoomId),
    #[error(transparent)]
    Player(#[from] InvalidPlayerId),
}

impl IntoResponse for ResourceDoesNotExist {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::NOT_FOUND,
            Json(json!({"error": self.to_string()})),
        )
            .into_response()
    }
}
