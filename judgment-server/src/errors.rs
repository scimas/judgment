use axum::{http::StatusCode, response::IntoResponse, Json};
use judgment::InvalidTransition;
use serde::Serialize;

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

#[derive(Debug, thiserror::Error, Serialize)]
#[error("invalid token")]
pub struct InvalidToken;

impl IntoResponse for InvalidToken {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::UNAUTHORIZED, Json(self)).into_response()
    }
}

#[derive(Debug, thiserror::Error, Serialize)]
#[error("no space left in server for another room")]
pub struct ServerFull;

impl IntoResponse for ServerFull {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::CONFLICT, Json(self)).into_response()
    }
}

#[derive(Debug, thiserror::Error, Serialize)]
#[error("no such room exists")]
pub struct InvalidRoomId;

impl IntoResponse for InvalidRoomId {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::NOT_FOUND, Json(self)).into_response()
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
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
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
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
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
        (StatusCode::NOT_FOUND, Json(self)).into_response()
    }
}
