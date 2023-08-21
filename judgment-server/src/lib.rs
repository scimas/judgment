pub mod errors;
mod room;
mod server;

use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use card_deck::standard_deck::{Card, Suit};
use errors::{InvalidRoomId, PlayError, ResourceDoesNotExist, RoomJoinError, ServerFull};
use judgment::Trick;
use pasetors::{keys::AsymmetricKeyPair, version4::V4};
use room::Action;
use serde::{Deserialize, Serialize};
use server::{AuthenticatedPlayer, Server};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use uuid::Uuid;

/// Create a router for Judgment.
pub fn judgment_router<P: AsRef<Path>>(
    key_pair: AsymmetricKeyPair<V4>,
    max_rooms: usize,
    frontend_path: P,
) -> (Router, Arc<RwLock<Server>>) {
    let server = Arc::new(RwLock::new(Server::new(key_pair, max_rooms)));

    let serve_dir = ServeDir::new(frontend_path);
    let router = Router::new()
        .route("/api/create_room", post(create_room))
        .route("/api/join", post(join))
        .route("/api/play", post(play))
        .route("/api/trick", get(trick))
        .route("/api/predictions", get(predictions))
        .route("/api/my_hand", get(hand_of_player))
        .route("/api/scores", get(scores))
        .route("/api/round_scores", get(round_scores))
        .route("/api/trump_suit", get(trump_suit))
        .fallback_service(serve_dir)
        .with_state(server.clone());

    (router, server)
}

async fn create_room(
    State(server): State<Arc<RwLock<Server>>>,
    Json(room_request): Json<NewRoomRequest>,
) -> Result<Json<RoomPayload>, ServerFull> {
    log::info!("received create room request");
    server
        .write()
        .await
        .create_room(
            room_request.players,
            room_request.starting_hand_size,
            room_request.decks,
        )
        .map(|room_id| Json(RoomPayload { room_id }))
}

async fn join(
    State(server): State<Arc<RwLock<Server>>>,
    Json(payload): Json<RoomPayload>,
) -> Result<Json<JoinSuccess>, RoomJoinError> {
    log::info!("received join request");
    server.write().await.join(&payload.room_id).map(|token| {
        Json(JoinSuccess {
            token_type: "Bearer".into(),
            token,
        })
    })
}

async fn play(
    player: AuthenticatedPlayer,
    State(server): State<Arc<RwLock<Server>>>,
    Json(action): Json<Action>,
) -> Result<StatusCode, PlayError> {
    log::info!("received play request from player {}", player.player_id);
    server
        .write()
        .await
        .play(action, player.player_id, &player.room_id)
        .map(|_| StatusCode::OK)
}

async fn trick(
    State(server): State<Arc<RwLock<Server>>>,
    Query(payload): Query<RoomPayload>,
) -> Result<Json<Trick>, InvalidRoomId> {
    log::info!("received trick request");
    let mut receiver = server
        .read()
        .await
        .room(&payload.room_id)?
        .trick_sender()
        .subscribe();
    let trick = {
        tokio::select! {
            _ = receiver.changed() => (),
            _ = tokio::time::sleep(Duration::from_secs(10)) => ()
        };
        receiver.borrow().clone()
    };
    Ok(Json(trick))
}

async fn predictions(
    State(server): State<Arc<RwLock<Server>>>,
    Query(payload): Query<RoomPayload>,
) -> Result<Json<Vec<Option<u8>>>, InvalidRoomId> {
    log::info!("received predictions request");
    let mut receiver = server
        .read()
        .await
        .room(&payload.room_id)?
        .predictions_sender()
        .subscribe();
    let predictions = {
        tokio::select! {
            _ = receiver.changed() => (),
            _ = tokio::time::sleep(Duration::from_secs(30)) => ()
        };
        receiver.borrow().clone()
    };
    Ok(Json(predictions))
}

async fn hand_of_player(
    player: AuthenticatedPlayer,
    State(server): State<Arc<RwLock<Server>>>,
) -> Result<Json<Vec<Card>>, ResourceDoesNotExist> {
    log::info!("received hand request from player {}", player.player_id);
    Ok(Json(
        server
            .read()
            .await
            .room(&player.room_id)?
            .hand_of_player(player.player_id)?
            .to_vec(),
    ))
}

async fn scores(
    State(server): State<Arc<RwLock<Server>>>,
    Query(payload): Query<RoomPayload>,
) -> Result<Json<Vec<i64>>, InvalidRoomId> {
    log::info!("received scores request");
    let mut receiver = server
        .read()
        .await
        .room(&payload.room_id)?
        .game_scores_sender()
        .subscribe();
    let scores = {
        tokio::select! {
            _ = receiver.changed() => (),
            _ = tokio::time::sleep(Duration::from_secs(120)) => ()
        };
        receiver.borrow().clone()
    };
    Ok(Json(scores))
}

async fn round_scores(
    State(server): State<Arc<RwLock<Server>>>,
    Query(payload): Query<RoomPayload>,
) -> Result<Json<Vec<u8>>, InvalidRoomId> {
    log::info!("received round scores request");
    let mut receiver = server
        .read()
        .await
        .room(&payload.room_id)?
        .round_scores_sender()
        .subscribe();
    let scores = {
        tokio::select! {
            _ = receiver.changed() => (),
            _ = tokio::time::sleep(Duration::from_secs(60)) => ()
        };
        receiver.borrow().clone()
    };
    Ok(Json(scores))
}

async fn trump_suit(
    State(server): State<Arc<RwLock<Server>>>,
    Query(payload): Query<RoomPayload>,
) -> Result<Json<Option<Suit>>, InvalidRoomId> {
    log::info!("received round scores request");
    let mut receiver = server
        .read()
        .await
        .room(&payload.room_id)?
        .trump_suit_sender()
        .subscribe();
    let suit = {
        tokio::select! {
            _ = receiver.changed() => (),
            _ = tokio::time::sleep(Duration::from_secs(120)) => ()
        };
        *receiver.borrow()
    };
    Ok(Json(suit))
}

#[derive(Debug, Serialize)]
struct JoinSuccess {
    token_type: String,
    token: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct RoomPayload {
    room_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct NewRoomRequest {
    players: u8,
    starting_hand_size: u8,
    decks: u8,
}
