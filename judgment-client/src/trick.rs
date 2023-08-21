use std::time::Duration;

use card_deck::standard_deck::Suit;
use either::Either;
use gloo_net::http::Request;
use uuid::Uuid;
use yew::{html, platform::time::sleep, Component, Html, Properties};

use crate::InvalidRoomId;

#[derive(Debug, PartialEq, Default)]
pub(crate) struct Trick {
    cards: judgment::Trick,
    trump_suit: Option<Suit>,
}

#[derive(Debug, PartialEq, Properties)]
pub(crate) struct Props {
    pub(crate) room_id: Uuid,
}

pub(crate) enum Msg {
    QueryTrick,
    TrickUpdated(judgment::Trick),
    DisplayError(String),
    QueryTrumpSuit,
    TrumpSuitUpdated(Option<Suit>),
}

impl Component for Trick {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &yew::Context<Self>) -> Self {
        ctx.link().send_message(Msg::QueryTrick);
        ctx.link().send_message(Msg::QueryTrumpSuit);
        Trick::default()
    }

    fn view(&self, _ctx: &yew::Context<Self>) -> yew::Html {
        html! {
            <>
                <p>{
                    match &self.trump_suit {
                        Some(suit) => suit.to_string(),
                        None => "None".to_string(),
                    }
                }</p>
                <div class="trick">
                    {
                        self.cards.iter().map(|opt_card| match opt_card {
                            None => html!{<div class="trick_card">{"\u{1f0a0}"}</div>},
                            Some(card) => {
                                let class = format!("trick_card {}", card.suit().unwrap().name());
                                html!{<div class={class}>{card.to_string()}</div>}
                            }
                        }).collect::<Html>()
                    }
                </div>
            </>
        }
    }

    fn update(&mut self, ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::QueryTrick => {
                let room_id = ctx.props().room_id;
                ctx.link().send_future(async move {
                    match query_trick(room_id).await {
                        Ok(trick) => Msg::TrickUpdated(trick),
                        Err(err) => Msg::DisplayError(err.to_string()),
                    }
                });
                false
            }
            Msg::TrickUpdated(trick) => {
                ctx.link().send_message(Msg::QueryTrick);
                if self.cards == trick {
                    false
                } else {
                    self.cards = trick;
                    true
                }
            }
            Msg::QueryTrumpSuit => {
                let room_id = ctx.props().room_id;
                ctx.link().send_future(async move {
                    match query_trump_suit(room_id).await {
                        Ok(suit) => Msg::TrumpSuitUpdated(suit),
                        Err(err) => Msg::DisplayError(err.to_string()),
                    }
                });
                false
            }
            Msg::TrumpSuitUpdated(suit) => {
                if self.trump_suit == suit {
                    ctx.link().send_future(async move {
                        sleep(Duration::from_secs(10)).await;
                        Msg::QueryTrumpSuit
                    });
                    false
                } else {
                    self.trump_suit = suit;
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

async fn query_trick(room_id: Uuid) -> Result<judgment::Trick, QueryError> {
    let response = Request::get("/judgment/api/trick")
        .query([("room_id", room_id.to_string())])
        .send()
        .await?;
    let body = response.text().await?;
    let mut json_deserializer = serde_json::Deserializer::from_str(&body);
    let deserialized: Either<judgment::Trick, InvalidRoomId> =
        either::serde_untagged::deserialize(&mut json_deserializer)?;
    match deserialized {
        Either::Left(trick) => Ok(trick),
        Either::Right(err) => Err(err.into()),
    }
}

#[derive(Debug, thiserror::Error)]
enum QueryError {
    #[error(transparent)]
    InvalidRoomId(#[from] InvalidRoomId),
    #[error(transparent)]
    NetworkError(#[from] gloo_net::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

async fn query_trump_suit(room_id: Uuid) -> Result<Option<Suit>, QueryError> {
    let response = Request::get("/judgment/api/trump_suit")
        .query([("room_id", room_id.to_string())])
        .send()
        .await?;
    let body = response.text().await?;
    let mut json_deserializer = serde_json::Deserializer::from_str(&body);
    let deserialized: Either<Option<Suit>, InvalidRoomId> =
        either::serde_untagged::deserialize(&mut json_deserializer)?;
    match deserialized {
        Either::Left(suit) => Ok(suit),
        Either::Right(err) => Err(err.into()),
    }
}
