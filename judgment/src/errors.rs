#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InvalidTransition {
    #[error("cannot deal cards before the game has started")]
    DealBeforeGameStart,
    #[error("cannot deal cards more than once in a round")]
    ReDeal,
    #[error("cannot play before the scores have been predicted")]
    PlayBeforeScorePrediction,
    #[error("cannot predict score before the cards have beel dealt")]
    PredictBeforeDeal,
    #[error("cannot predict scores more than once in a round")]
    RePredict,
    #[error("cannot take any action after game is over")]
    GameOver,
    #[error("game is already in play")]
    Restart,
    #[error("not this player's turn")]
    OutOfTurnPlay,
    #[error("predicted score is impossible to achieve")]
    PredictionOutOfRange,
    #[error("last player to predict score in the round; score not allowed")]
    LastPlayerPrediction,
    #[error("the player does not have the played card")]
    NoSuchPlayerCard,
    #[error("must match the suit of the first card when possible")]
    SuitMismatch,
}
