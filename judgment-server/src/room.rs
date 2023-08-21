use card_deck::standard_deck::{Card, Suit};
use judgment::{InvalidTransition, Judgment, StateUpdate, Transition, Trick};
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
    predictions_sender: watch::Sender<Vec<Option<u8>>>,
    round_scores_sender: watch::Sender<Vec<u8>>,
    game_scores_sender: watch::Sender<Vec<i64>>,
    trump_suit_sender: watch::Sender<Option<Suit>>,
}

impl Room {
    /// Create a new room that can accommodate given amount of players and card
    /// decks.
    pub fn new(players: u8, starting_hand_size: u8, decks: u8) -> Self {
        let game = Judgment::new(players, starting_hand_size, Some(decks));
        let (trick_sender, _) = watch::channel(game.trick().clone());
        let (predictions_sender, _) = watch::channel(Vec::new());
        let (round_scores_sender, _) = watch::channel(Vec::new());
        let (game_scores_sender, _) = watch::channel(Vec::new());
        let (trump_suit_sender, _) = watch::channel(None);
        Room {
            joined_players: 0,
            game,
            max_players: players,
            trick_sender,
            predictions_sender,
            round_scores_sender,
            game_scores_sender,
            trump_suit_sender,
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
            self.game.start().unwrap();
            self.trump_suit_sender
                .send_replace(self.game.trump_suit().cloned());
            self.play(Action::Deal, usize::from(self.max_players))
                .unwrap();
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
                let updates = self.game.update(transition)?;
                assert!(
                    !updates.is_empty() && updates.len() <= 3,
                    "a Transition::Play should create 1..=3 updates"
                );
                for update in updates {
                    match update {
                        StateUpdate::Trick(trick) => {
                            self.trick_sender.send_replace(trick);
                        }
                        StateUpdate::RoundScores(scores) => {
                            self.round_scores_sender.send_replace(scores);
                        }
                        StateUpdate::GameScores(scores) => {
                            self.game_scores_sender.send_replace(scores);
                            self.trump_suit_sender
                                .send_replace(self.game.trump_suit().cloned());
                        }
                        StateUpdate::CardsDealt | StateUpdate::Predictions(_) => {
                            unreachable!("a Transition::Play should never result in this state")
                        }
                    }
                }
                Ok(())
            }
            Action::PredictScore(score) => {
                let transition = Transition::PredictScore { player, score };
                let updates = self.game.update(transition)?;
                assert_eq!(
                    updates.len(),
                    1,
                    "a Transition::PredictScore should create exactly 1 update"
                );
                for update in updates {
                    match update {
                        StateUpdate::Predictions(predictions) => {
                            self.predictions_sender.send_replace(predictions);
                        }
                        _ => unreachable!(
                            "a Transition::PredictScore should never result in this state"
                        ),
                    }
                }
                Ok(())
            }
            Action::Deal => {
                let seed = rand::random();
                let transition = Transition::Deal { seed };
                let updates = self.game.update(transition)?;
                assert_eq!(
                    updates.len(),
                    1,
                    "a Transition::Deal should create exactly 1 update"
                );
                for update in updates {
                    match update {
                        StateUpdate::CardsDealt => (),
                        _ => unreachable!("a Transition::Deal should never result in this state"),
                    }
                }
                Ok(())
            }
        }
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
    pub fn predictions_sender(&self) -> &watch::Sender<Vec<Option<u8>>> {
        &self.predictions_sender
    }

    /// Get the notifier channel that communicates when the round scores change.
    pub fn round_scores_sender(&self) -> &watch::Sender<Vec<u8>> {
        &self.round_scores_sender
    }

    /// Get the notifier channel that communicates when the game scores change.
    pub fn game_scores_sender(&self) -> &watch::Sender<Vec<i64>> {
        &self.game_scores_sender
    }

    /// Check whether the game is over.
    pub fn is_game_over(&self) -> bool {
        self.game.is_over()
    }

    pub fn trump_suit_sender(&self) -> &watch::Sender<Option<Suit>> {
        &self.trump_suit_sender
    }
}

/// An action that a player can take; either play a card or pass their turn.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Action {
    Play(Card),
    PredictScore(u8),
    Deal,
}

#[cfg(test)]
mod tests {
    use pasetors::claims::Claims;

    use crate::errors::RoomFull;

    use super::Room;

    #[test]
    fn test_room_joining() {
        let mut room = Room::new(2, 2, 1);
        for _ in 0..2 {
            assert!(matches!(room.join(), Ok(Claims { .. })));
        }
        assert!(matches!(room.join(), Err(RoomFull { .. })));
    }
}
