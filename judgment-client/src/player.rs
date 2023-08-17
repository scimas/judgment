use std::{collections::HashMap, time::Duration};

use card_deck::standard_deck::{Card, Suit};
use either::Either;
use gloo_net::http::Request;
use judgment::InvalidTransition;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::{html, platform::time::sleep, Component, Html, InputEvent, Properties};

use crate::{InvalidPlayerId, InvalidRoomId};

#[derive(Debug, PartialEq)]
pub(crate) struct Player {
    hand: HashMap<Suit, Vec<Card>>,
    prediction_input: Option<u8>,
}

impl Default for Player {
    fn default() -> Self {
        let hand = Suit::all_suits()
            .into_iter()
            .map(|suit| (suit, Vec::new()))
            .collect();
        Player {
            hand,
            prediction_input: None,
        }
    }
}

#[derive(Debug, PartialEq, Properties)]
pub(crate) struct Props {
    pub(crate) room_id: Uuid,
    pub(crate) token: String,
}

pub(crate) enum Msg {
    Play(Card),
    QueryHand,
    HandUpdated(HashMap<Suit, Vec<Card>>),
    PredictionInput(u8),
    Predict,
    Deal,
    DisplayError(String),
}

impl Component for Player {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &yew::Context<Self>) -> Self {
        ctx.link().send_message(Msg::QueryHand);
        Player::default()
    }

    fn view(&self, ctx: &yew::Context<Self>) -> yew::Html {
        let prediction_input_callback = ctx.link().callback(|event: InputEvent| {
            let target = event.target().unwrap();
            let input_element = target.unchecked_into::<HtmlInputElement>();
            Msg::PredictionInput(input_element.value().parse().unwrap())
        });
        let predict_callback = ctx.link().callback(|_| Msg::Predict);
        let deal_callback = ctx.link().callback(|_| Msg::Deal);
        html! {
            <>
                <div class="hand">
                {
                    self.hand.iter().map(|(suit, cards)| html!{
                        <div class={format!("hand_stack {}", suit.name())}>
                            {
                                cards.iter().map(|card| {
                                    let card = *card;
                                    html!{<button class="playable" onclick={ctx.link().callback(move |_| Msg::Play(card))}>{card.to_string()}</button>}}).collect::<Html>()
                            }
                        </div>
                    }).collect::<Html>()
                }
                </div>
                <div class="prediction">
                    <label for="players">{"Score Prediction: "}</label>
                    <input type="number" id="prediction" min=0 max=13 placeholder="Score Prediction" oninput={prediction_input_callback}/>
                    <button type="button" onclick={predict_callback}>{"Predict"}</button>
                </div>
                <button type="button" onclick={deal_callback}>{"Deal"}</button>
            </>
        }
    }

    fn update(&mut self, ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Play(card) => {
                let token = ctx.props().token.clone();
                ctx.link().send_future(async move {
                    match play(&token, &Action::Play(card)).await {
                        Ok(_) => Msg::QueryHand,
                        Err(PlayError::Action(err)) => Msg::DisplayError(err.to_string()),
                        Err(PlayError::Network(_) | PlayError::Serde(_)) => Msg::DisplayError(
                            "server or network related issue, try again after some time"
                                .to_string(),
                        ),
                    }
                });
                false
            }
            Msg::QueryHand => {
                let token = ctx.props().token.clone();
                ctx.link().send_future(async move {
                    match query_hand(&token).await {
                        Ok(hand) => Msg::HandUpdated(hand),
                        Err(QueryHandError::ResourceDoesNotExist(err)) => {
                            Msg::DisplayError(err.to_string())
                        }
                        Err(QueryHandError::NetworkError(_) | QueryHandError::SerdeError(_)) => {
                            Msg::DisplayError(
                                "server or network related issue, try again after some time"
                                    .to_string(),
                            )
                        }
                    }
                });
                false
            }
            Msg::HandUpdated(hand) => {
                if self.hand == hand {
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(5)).await;
                        Msg::QueryHand
                    });
                    false
                } else {
                    self.hand = hand;
                    true
                }
            }
            Msg::DisplayError(err) => {
                gloo_dialogs::alert(&err);
                false
            }
            Msg::PredictionInput(score) => {
                self.prediction_input = Some(score);
                false
            }
            Msg::Predict => {
                if let Some(score) = self.prediction_input {
                    let token = ctx.props().token.clone();
                    ctx.link().send_future(async move {
                        match play(&token, &Action::PredictScore(score)).await {
                            Ok(_) => Msg::QueryHand,
                            Err(PlayError::Action(err)) => Msg::DisplayError(err.to_string()),
                            Err(PlayError::Network(_) | PlayError::Serde(_)) => Msg::DisplayError(
                                "server or network related issue, try again after some time"
                                    .to_string(),
                            ),
                        }
                    });
                } else {
                    ctx.link()
                        .send_message(Msg::DisplayError("prediction cannot be empty".to_string()));
                }
                false
            }
            Msg::Deal => {
                let token = ctx.props().token.clone();
                ctx.link().send_future(async move {
                    match play(&token, &Action::Deal).await {
                        Ok(_) => Msg::QueryHand,
                        Err(PlayError::Action(err)) => Msg::DisplayError(err.to_string()),
                        Err(PlayError::Network(_) | PlayError::Serde(_)) => Msg::DisplayError(
                            "server or network related issue, try again after some time"
                                .to_string(),
                        ),
                    }
                });
                false
            }
        }
    }
}

async fn query_hand(token: &str) -> Result<HashMap<Suit, Vec<Card>>, QueryHandError> {
    let response = Request::get("/judgment/api/my_hand")
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await?;
    let body = response.text().await?;
    let mut json_deserializer = serde_json::Deserializer::from_str(&body);
    let deserialized: Either<Vec<Card>, ResourceDoesNotExist> =
        either::serde_untagged::deserialize(&mut json_deserializer)?;
    match deserialized {
        Either::Left(cards) => {
            let hand: HashMap<Suit, Vec<Card>> = Suit::all_suits()
                .into_iter()
                .map(|suit| {
                    (
                        suit,
                        cards
                            .iter()
                            .filter(|card| card.suit().unwrap() == &suit)
                            .cloned()
                            .collect::<Vec<Card>>(),
                    )
                })
                .collect();
            Ok(hand)
        }
        Either::Right(err) => Err(err.into()),
    }
}

#[derive(Debug, thiserror::Error, Deserialize)]
enum ResourceDoesNotExist {
    #[error(transparent)]
    Room(#[from] InvalidRoomId),
    #[error(transparent)]
    Player(#[from] InvalidPlayerId),
}

#[derive(Debug, thiserror::Error)]
enum QueryHandError {
    #[error(transparent)]
    ResourceDoesNotExist(#[from] ResourceDoesNotExist),
    #[error(transparent)]
    NetworkError(#[from] gloo_net::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

async fn play(token: &str, action: &Action) -> Result<(), PlayError> {
    let response = Request::post("/judgment/api/play")
        .header("Authorization", &format!("Bearer {token}"))
        .json(action)?
        .send()
        .await?;
    if response.ok() {
        return Ok(());
    }
    let error: ActionError = response.json().await?;
    Err(error.into())
}

#[derive(Debug, Serialize)]
enum Action {
    Play(Card),
    PredictScore(u8),
    Deal,
}

#[derive(Debug, thiserror::Error, Deserialize)]
enum ActionError {
    #[error(transparent)]
    InvalidRoomId(#[from] InvalidRoomId),
    #[error(transparent)]
    InvalidTransition(#[from] InvalidTransition),
}

#[derive(Debug, thiserror::Error)]
enum PlayError {
    #[error(transparent)]
    Action(#[from] ActionError),
    #[error(transparent)]
    Network(#[from] gloo_net::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}
