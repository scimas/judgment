use card_deck::standard_deck::Card;
use judgment::{InvalidTransition, Judgment, Transition, Trick};
use pasetors::claims::Claims;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::errors::{InvalidPlayerId, RoomFull};

#[derive(Debug)]
pub struct Room {
    joined_players: u8,
    game: Judgment,
    max_players: u8,
    trick_sender: watch::Sender<Trick>,
    predictions_sender: watch::Sender<Vec<u8>>,
    last_move: Option<Action>,
}

impl Room {
    /// Create a new room that can accommodate given amount of players and card
    /// decks.
    pub fn new(players: u8, starting_hand_size: u8, decks: u8) -> Self {
        let game = Judgment::new(players, starting_hand_size, Some(decks));
        let (trick_sender, _) = watch::channel(game.trick().clone());
        let (prediction_sender, _) = watch::channel(Vec::new());
        Room {
            joined_players: 0,
            game,
            max_players: players,
            trick_sender,
            predictions_sender: prediction_sender,
            last_move: None,
        }
    }

    /// Try to join the room.
    pub fn join(&mut self) -> Result<Claims, RoomFull> {
        if self.is_full() {
            return Err(RoomFull {
                max_players: self.max_players,
            });
        }
        let mut claim = Claims::new().unwrap();
        claim.subject(&self.joined_players.to_string()).unwrap();
        self.joined_players += 1;
        if self.is_full() {
            let seed = rand::random();
            self.game.update(Transition::Deal { seed }).unwrap();
        }
        Ok(claim)
    }

    /// Check whether the room's player capacity is full.
    pub fn is_full(&self) -> bool {
        self.max_players == self.joined_players
    }

    /// Attempt to play a card.
    pub fn play(&mut self, action: Action, player: usize) -> Result<(), InvalidTransition> {
        if !self.is_full() {
            return Err(InvalidTransition::OutOfTurnPlay);
        }
        match action {
            Action::Play(card) => {
                let transition = Transition::Play { player, card };
                match self.game.update(transition) {
                    Ok(_) => {
                        self.trick_sender.send_replace(self.trick().clone());
                        self.last_move = Some(action);
                        Ok(())
                    }
                    err @ Err(_) => err,
                }
            }
            Action::PredictScore(score) => {
                let transition = Transition::PredictScore { player, score };
                match self.game.update(transition) {
                    Ok(_) => {
                        self.predictions_sender
                            .send_replace(self.game.predicted_scores().unwrap());
                        Ok(())
                    }
                    err @ Err(_) => err,
                }
            }
        }
    }

    /// Get the room's playing area.
    pub fn trick(&self) -> &Trick {
        self.game.trick()
    }

    pub fn predictions(&self) -> Option<Vec<u8>> {
        self.game.predicted_scores()
    }
    /// Get the hand of a player.
    pub fn hand_of_player(&self, player: usize) -> Result<&[Card], InvalidPlayerId> {
        self.game.hand_of_player(player).ok_or(InvalidPlayerId)
    }

    /// Get the notifier channel that communicates when the trick changes.
    pub fn trick_sender(&self) -> &watch::Sender<Trick> {
        &self.trick_sender
    }

    /// Get the notifier channel that communicates when the predictions change.
    pub fn predictions_sender(&self) -> &watch::Sender<Vec<u8>> {
        &self.predictions_sender
    }

    /// Check whether the game is over.
    pub fn is_game_over(&self) -> bool {
        self.game.is_over()
    }

    /// Get the last played valid move.
    pub fn last_move(&self) -> Option<&Action> {
        self.last_move.as_ref()
    }
}

/// An action that a player can take; either play a card or pass their turn.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Action {
    Play(Card),
    PredictScore(u8),
}
