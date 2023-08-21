use std::{iter, time::Duration};

use either::Either;
use gloo_net::http::Request;
use uuid::Uuid;
use yew::{html, platform::time::sleep, Component, Html, Properties};

use crate::InvalidRoomId;

#[derive(Debug, PartialEq, Default)]
pub(crate) struct Scores {
    predictions: Vec<Option<u8>>,
    round_scores: Vec<u8>,
    scores: Vec<i64>,
}

pub(crate) enum Msg {
    QueryPredictions,
    PredictionsUpdated(Vec<Option<u8>>),
    QueryScores,
    ScoresUpdated(Vec<i64>),
    QueryRoundScores,
    RoundScoresUpdated(Vec<u8>),
    DisplayError(String),
}

#[derive(Debug, PartialEq, Properties)]
pub(crate) struct Props {
    pub(crate) room_id: Uuid,
}

impl Component for Scores {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &yew::Context<Self>) -> Self {
        ctx.link().send_message(Msg::QueryPredictions);
        ctx.link().send_message(Msg::QueryScores);
        ctx.link().send_message(Msg::QueryRoundScores);
        Scores::default()
    }

    fn view(&self, _ctx: &yew::Context<Self>) -> yew::Html {
        html! {
            <div>
                <details class="scores" open=true>
                    <summary>{"Predictions"}</summary>
                    <table>
                        <thead>
                            <tr>
                                {iter::once(html!{<th scope="row">{"Player"}</th>}).chain((0..self.predictions.len()).map(|idx| html!{<th scope="col">{idx}</th>})).collect::<Html>()}
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                {iter::once(html!{<th scope="row">{"Prediction"}</th>}).chain(self.predictions.iter().map(|pred| html!{<td>{pred.map(|s| s.to_string()).unwrap_or("-".to_string())}</td>})).collect::<Html>()}
                            </tr>
                        </tbody>
                    </table>
                </details>
                <details class="scores" open=true>
                    <summary>{"Round Scores"}</summary>
                    <table>
                        <thead>
                            <tr>
                                {iter::once(html!{<th scope="row">{"Player"}</th>}).chain((0..self.round_scores.len()).map(|idx| html!{<th scope="col">{idx}</th>})).collect::<Html>()}
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                {iter::once(html!{<th scope="row">{"Score"}</th>}).chain(self.round_scores.iter().map(|score| html!{<td>{score}</td>})).collect::<Html>()}
                            </tr>
                        </tbody>
                    </table>
                </details>
                <details class="scores">
                    <summary>{"Game Scores"}</summary>
                    <table>
                        <thead>
                            <tr>
                                {iter::once(html!{<th scope="row">{"Player"}</th>}).chain((0..self.scores.len()).map(|idx| html!{<th scope="col">{idx}</th>})).collect::<Html>()}
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                {iter::once(html!{<th scope="row">{"Score"}</th>}).chain(self.scores.iter().map(|score| html!{<td>{score}</td>})).collect::<Html>()}
                            </tr>
                        </tbody>
                    </table>
                </details>
            </div>
        }
    }

    fn update(&mut self, ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::QueryPredictions => {
                let room_id = ctx.props().room_id;
                ctx.link().send_future(async move {
                    match query_predictions(room_id).await {
                        Ok(predictions) => Msg::PredictionsUpdated(predictions),
                        Err(QueryScoresError::InvalidRoomId(err)) => {
                            Msg::DisplayError(err.to_string())
                        }
                        Err(QueryScoresError::Network(_) | QueryScoresError::Serde(_)) => {
                            Msg::DisplayError(
                                "server or network related issue, try again after some time"
                                    .to_string(),
                            )
                        }
                    }
                });
                false
            }
            Msg::PredictionsUpdated(predictions) => {
                if self.predictions == predictions {
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(5)).await;
                        Msg::QueryPredictions
                    });
                    false
                } else {
                    self.predictions = predictions;
                    ctx.link().send_message(Msg::QueryPredictions);
                    true
                }
            }
            Msg::QueryScores => {
                let room_id = ctx.props().room_id;
                ctx.link().send_future(async move {
                    match query_scores(room_id).await {
                        Ok(scores) => Msg::ScoresUpdated(scores),
                        Err(QueryScoresError::InvalidRoomId(err)) => {
                            Msg::DisplayError(err.to_string())
                        }
                        Err(QueryScoresError::Network(_) | QueryScoresError::Serde(_)) => {
                            Msg::DisplayError(
                                "server or network related issue, try again after some time"
                                    .to_string(),
                            )
                        }
                    }
                });
                false
            }
            Msg::ScoresUpdated(scores) => {
                if self.scores == scores {
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(5)).await;
                        Msg::QueryScores
                    });
                    false
                } else {
                    self.scores = scores;
                    ctx.link().send_message(Msg::QueryScores);
                    true
                }
            }
            Msg::QueryRoundScores => {
                let room_id = ctx.props().room_id;
                ctx.link().send_future(async move {
                    match query_round_scores(room_id).await {
                        Ok(Some(scores)) => Msg::RoundScoresUpdated(scores),
                        Ok(None) => Msg::RoundScoresUpdated(Vec::new()),
                        Err(QueryScoresError::InvalidRoomId(err)) => {
                            Msg::DisplayError(err.to_string())
                        }
                        Err(QueryScoresError::Network(_) | QueryScoresError::Serde(_)) => {
                            Msg::DisplayError(
                                "server or network related issue, try again after some time"
                                    .to_string(),
                            )
                        }
                    }
                });
                false
            }
            Msg::RoundScoresUpdated(scores) => {
                if self.round_scores == scores {
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(5)).await;
                        Msg::QueryRoundScores
                    });
                    false
                } else {
                    self.round_scores = scores;
                    ctx.link().send_message(Msg::QueryRoundScores);
                    true
                }
            }
            Msg::DisplayError(err) => {
                gloo_dialogs::alert(&err);
                false
            }
        }
    }
}

async fn query_scores(room_id: Uuid) -> Result<Vec<i64>, QueryScoresError> {
    let response = Request::get("/judgment/api/scores")
        .query([("room_id", room_id.to_string())])
        .send()
        .await?;
    let body = response.text().await?;
    let mut json_deserializer = serde_json::Deserializer::from_str(&body);
    let deserialized: Either<Vec<i64>, InvalidRoomId> =
        either::serde_untagged::deserialize(&mut json_deserializer)?;
    match deserialized {
        Either::Left(scores) => Ok(scores),
        Either::Right(err) => Err(err.into()),
    }
}

#[derive(Debug, thiserror::Error)]
enum QueryScoresError {
    #[error(transparent)]
    InvalidRoomId(#[from] InvalidRoomId),
    #[error(transparent)]
    Network(#[from] gloo_net::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

async fn query_predictions(room_id: Uuid) -> Result<Vec<Option<u8>>, QueryScoresError> {
    let response = Request::get("/judgment/api/predictions")
        .query([("room_id", room_id.to_string())])
        .send()
        .await?;
    let body = response.text().await?;
    let mut json_deserializer = serde_json::Deserializer::from_str(&body);
    let deserialized: Either<Vec<Option<u8>>, InvalidRoomId> =
        either::serde_untagged::deserialize(&mut json_deserializer)?;
    match deserialized {
        Either::Left(predictions) => Ok(predictions),
        Either::Right(err) => Err(err.into()),
    }
}

async fn query_round_scores(room_id: Uuid) -> Result<Option<Vec<u8>>, QueryScoresError> {
    let response = Request::get("/judgment/api/round_scores")
        .query([("room_id", room_id.to_string())])
        .send()
        .await?;
    let body = response.text().await?;
    let mut json_deserializer = serde_json::Deserializer::from_str(&body);
    let deserialized: Either<Option<Vec<u8>>, InvalidRoomId> =
        either::serde_untagged::deserialize(&mut json_deserializer)?;
    match deserialized {
        Either::Left(scores) => Ok(scores),
        Either::Right(err) => Err(err.into()),
    }
}
