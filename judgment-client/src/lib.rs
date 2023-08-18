use serde::Deserialize;

pub mod app;
mod player;
mod scores;
mod trick;

#[derive(Debug, thiserror::Error, Deserialize)]
#[error("no such room exists")]
pub(crate) struct InvalidRoomId;

#[derive(Debug, thiserror::Error, Deserialize)]
#[error("not a valid player Id")]
pub(crate) struct InvalidPlayerId;
