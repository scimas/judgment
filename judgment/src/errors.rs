#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid transition for the current stage of the game")]
    InvalidTransition,
}
