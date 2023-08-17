use std::time::Duration;

use either::Either;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::{html, platform::time::sleep, Component, Html, InputEvent};

use crate::{player::Player, trick::Trick, InvalidRoomId};

pub struct App {
    room_id: Option<Uuid>,
    token: Option<String>,
    room_id_input: Option<String>,
    players_input: Option<u8>,
    hand_size_input: Option<u8>,
    decks_input: Option<u8>,
    scores: Vec<i64>,
    predictions: Vec<Option<u8>>,
    round_scores: Vec<u8>,
}

pub enum Msg {
    RoomIdInput(String),
    JoinRoom,
    PlayersInput(u8),
    HandSizeInput(u8),
    DecksInput(u8),
    CreateRoom,
    DisplayError(String),
    JoinedRoom(Auth, RoomPayload),
    CreatedRoom(RoomPayload),
    QueryScores,
    ScoresUpdated(Vec<i64>),
    QueryPredictions,
    PredictionsUpdated(Vec<Option<u8>>),
    QueryRoundScores,
    RoundScoresUpdated(Vec<u8>),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &yew::Context<Self>) -> Self {
        ctx.link().send_message(Msg::QueryScores);
        ctx.link().send_message(Msg::QueryPredictions);
        ctx.link().send_message(Msg::QueryRoundScores);
        Self {
            room_id: None,
            token: None,
            room_id_input: None,
            players_input: None,
            hand_size_input: None,
            decks_input: None,
            scores: Vec::new(),
            predictions: Vec::new(),
            round_scores: Vec::new(),
        }
    }

    fn view(&self, ctx: &yew::Context<Self>) -> Html {
        if let Some(room_id) = self.room_id {
            if let Some(token) = &self.token {
                html! {
                    <div class="app">
                        <Trick room_id={room_id}/>
                        <Player room_id={room_id} token={token.clone()}/>
                        <details class="scores" open=true>
                            <summary>{"Predictions"}</summary>
                            <table>
                                <thead>
                                    <tr>
                                        {(0..self.predictions.len()).map(|idx| html!{<th scope="col">{idx}</th>}).collect::<Html>()}
                                    </tr>
                                </thead>
                                <tbody>
                                    <tr>
                                        {self.predictions.iter().map(|pred| html!{<td>{pred.map(|s| s.to_string()).unwrap_or("-".to_string())}</td>}).collect::<Html>()}
                                    </tr>
                                </tbody>
                            </table>
                        </details>
                        <details class="scores" open=true>
                            <summary>{"Round Scores"}</summary>
                            <table>
                                <thead>
                                    <tr>
                                        {(0..self.round_scores.len()).map(|idx| html!{<th scope="col">{idx}</th>}).collect::<Html>()}
                                    </tr>
                                </thead>
                                <tbody>
                                    <tr>
                                        {self.round_scores.iter().map(|score| html!{<td>{score}</td>}).collect::<Html>()}
                                    </tr>
                                </tbody>
                            </table>
                        </details>
                        <details class="scores">
                            <summary>{"Scores"}</summary>
                            <table>
                                <thead>
                                    <tr>
                                        {(0..self.scores.len()).map(|idx| html!{<th scope="col">{idx}</th>}).collect::<Html>()}
                                    </tr>
                                </thead>
                                <tbody>
                                    <tr>
                                        {self.scores.iter().map(|score| html!{<td>{score}</td>}).collect::<Html>()}
                                    </tr>
                                </tbody>
                            </table>
                        </details>
                        <details>
                            <summary>{"Room ID"}</summary>
                            {room_id}
                        </details>
                    </div>
                }
            } else {
                html! {}
            }
        } else {
            let room_id_input_callback = ctx.link().callback(|event: InputEvent| {
                let target = event.target().unwrap();
                let input_element = target.unchecked_into::<HtmlInputElement>();
                Msg::RoomIdInput(input_element.value())
            });
            let join_callback = ctx.link().callback(|_| Msg::JoinRoom);
            let players_input_callback = ctx.link().callback(|event: InputEvent| {
                let target = event.target().unwrap();
                let input_element = target.unchecked_into::<HtmlInputElement>();
                Msg::PlayersInput(input_element.value().parse().unwrap())
            });
            let hand_size_input_callback = ctx.link().callback(|event: InputEvent| {
                let target = event.target().unwrap();
                let input_element = target.unchecked_into::<HtmlInputElement>();
                Msg::HandSizeInput(input_element.value().parse().unwrap())
            });
            let decks_input_callback = ctx.link().callback(|event: InputEvent| {
                let target = event.target().unwrap();
                let input_element = target.unchecked_into::<HtmlInputElement>();
                Msg::DecksInput(input_element.value().parse().unwrap())
            });
            let create_callback = ctx.link().callback(|_| Msg::CreateRoom);
            html! {
                <div class="app">
                    <label for="room_id">{"Room ID: "}</label>
                    <input type="text" id="room_id" minlength=32 maxlength=36 size=40 placeholder="Room ID to join existing room" oninput={room_id_input_callback}/>
                    <br/>
                    <button type="button" onclick={join_callback}>{"Join"}</button>
                    <br/>
                    <label for="players">{"Players: "}</label>
                    <input type="number" id="players" min=2 max=12 placeholder="Number of players" oninput={players_input_callback}/>
                    <label for="hand_size">{"Starting Hand Size: "}</label>
                    <input type="number" id="hand_size" min=1 max=13 placeholder="Starting hand size" oninput={hand_size_input_callback}/>
                    <label for="decks">{"Decks: "}</label>
                    <input type="number" id="decks" min=1 max=4 placeholder="Number of card decks" oninput={decks_input_callback}/>
                    <br/>
                    <button type="button" onclick={create_callback}>{"Create Room"}</button>
                </div>
            }
        }
    }

    fn update(&mut self, ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::RoomIdInput(s) => {
                self.room_id_input = Some(s);
                false
            }
            Msg::JoinRoom => {
                if let Some(room_id) = &self.room_id_input {
                    if let Ok(room_id) = Uuid::parse_str(room_id) {
                        let payload = RoomPayload { room_id };
                        ctx.link().send_future(async move {
                            match join_room(payload).await {
                                Ok(auth) => {
                                    Msg::JoinedRoom(auth, payload)
                                }
                                Err(JoinRoomError::RoomJoin(err)) => {
                                    Msg::DisplayError(err.to_string())
                                }
                                Err(JoinRoomError::Serde(_) | JoinRoomError::Network(_)) => {
                                    Msg::DisplayError(
                                    "server or network related issue, try again after some time"
                                        .to_string(),
                                )
                                }
                            }
                        });
                    } else {
                        ctx.link()
                            .send_message(Msg::DisplayError("room ID is invalid".to_string()));
                    }
                } else {
                    ctx.link()
                        .send_message(Msg::DisplayError("room ID cannot be empty".to_string()));
                }
                false
            }
            Msg::PlayersInput(n) => {
                self.players_input = Some(n);
                false
            }
            Msg::HandSizeInput(n) => {
                self.hand_size_input = Some(n);
                false
            }
            Msg::DecksInput(n) => {
                self.decks_input = Some(n);
                false
            }
            Msg::CreateRoom => {
                match (
                    &self.players_input,
                    &self.hand_size_input,
                    &self.decks_input,
                ) {
                    (Some(players_input), Some(hand_size_input), Some(decks_input)) => {
                        let players = *players_input;
                        let starting_hand_size = *hand_size_input;
                        let decks = *decks_input;
                        ctx.link().send_future(async move {
                            match create_room(players, starting_hand_size, decks).await {
                                Ok(room) => Msg::CreatedRoom(room),
                                Err(CreateRoomError::ServerFull(err)) => {
                                    Msg::DisplayError(err.to_string())
                                }
                                Err(
                                    CreateRoomError::SerdeError(_)
                                    | CreateRoomError::NetworkError(_),
                                ) => Msg::DisplayError(
                                    "server or network related issue, try again after some time"
                                        .to_string(),
                                ),
                            }
                        });
                    }
                    (_, _, _) => {
                        ctx.link().send_message(Msg::DisplayError("all of number of players, starting hand size and number of decks inputs must be provided".to_string()));
                    }
                }
                false
            }
            Msg::DisplayError(err) => {
                gloo_dialogs::alert(&err);
                false
            }
            Msg::JoinedRoom(auth, room) => {
                self.token = Some(auth.token);
                self.room_id = Some(room.room_id);
                true
            }
            Msg::CreatedRoom(room) => {
                self.room_id_input = Some(room.room_id.to_string());
                self.room_id = Some(room.room_id);
                ctx.link().send_message(Msg::JoinRoom);
                false
            }
            Msg::QueryScores => {
                if let Some(room_id) = self.room_id {
                    let room_id = room_id;
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
                } else {
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(15)).await;
                        Msg::QueryScores
                    });
                }
                false
            }
            Msg::ScoresUpdated(scores) => {
                ctx.link().send_future(async move {
                    sleep(Duration::from_secs(15)).await;
                    Msg::QueryScores
                });
                if self.scores == scores {
                    false
                } else {
                    self.scores = scores;
                    true
                }
            }
            Msg::QueryPredictions => {
                if let Some(room_id) = self.room_id {
                    let room_id = room_id;
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
                } else {
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(15)).await;
                        Msg::QueryPredictions
                    });
                }
                false
            }
            Msg::PredictionsUpdated(predictions) => {
                if self.predictions == predictions {
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(15)).await;
                        Msg::QueryPredictions
                    });
                    false
                } else {
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(5)).await;
                        Msg::QueryPredictions
                    });
                    self.predictions = predictions;
                    true
                }
            }
            Msg::QueryRoundScores => {
                if let Some(room_id) = self.room_id {
                    let room_id = room_id;
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
                } else {
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(15)).await;
                        Msg::QueryRoundScores
                    });
                }
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
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(15)).await;
                        Msg::QueryRoundScores
                    });
                    self.round_scores = scores;
                    true
                }
            }
        }
    }
}

async fn create_room(
    players: u8,
    starting_hand_size: u8,
    decks: u8,
) -> Result<RoomPayload, CreateRoomError> {
    let response = Request::post("/judgment/api/create_room")
        .json(&json!({ "players": players, "starting_hand_size": starting_hand_size, "decks": decks }))?
        .send()
        .await?;
    let body = response.text().await?;
    let mut json_deserializer = serde_json::Deserializer::from_str(&body);
    let deserialized: Either<RoomPayload, ServerFull> =
        either::serde_untagged::deserialize(&mut json_deserializer)?;
    match deserialized {
        Either::Left(room) => Ok(room),
        Either::Right(err) => Err(err.into()),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct RoomPayload {
    room_id: Uuid,
}

#[derive(Debug, thiserror::Error, Deserialize)]
#[error("server's room capacity is full")]
struct ServerFull;

#[derive(Debug, thiserror::Error)]
enum CreateRoomError {
    #[error(transparent)]
    ServerFull(#[from] ServerFull),
    #[error(transparent)]
    NetworkError(#[from] gloo_net::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

async fn join_room(payload: RoomPayload) -> Result<Auth, JoinRoomError> {
    let response = Request::post("/judgment/api/join")
        .json(&payload)?
        .send()
        .await?;
    let body = response.text().await?;
    let mut json_deserializer = serde_json::Deserializer::from_str(&body);
    let deserialized: Either<Auth, RoomJoinError> =
        either::serde_untagged::deserialize(&mut json_deserializer)?;
    match deserialized {
        Either::Left(auth) => Ok(auth),
        Either::Right(err) => Err(err.into()),
    }
}

#[derive(Debug, Deserialize)]
pub struct Auth {
    #[allow(dead_code)]
    token_type: String,
    token: String,
}

#[derive(Debug, thiserror::Error, Deserialize)]
#[error("room is full, capacity: {max_players}")]
struct RoomFull {
    max_players: u8,
}

#[derive(Debug, thiserror::Error, Deserialize)]
enum RoomJoinError {
    #[error(transparent)]
    InvalidRoomId(#[from] InvalidRoomId),
    #[error(transparent)]
    RoomFull(#[from] RoomFull),
}

#[derive(Debug, thiserror::Error)]
enum JoinRoomError {
    #[error(transparent)]
    RoomJoin(#[from] RoomJoinError),
    #[error(transparent)]
    Network(#[from] gloo_net::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
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
