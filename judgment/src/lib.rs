use card_deck::standard_deck::{Card, Rank, StandardDeckBuilder, Suit};
pub use errors::InvalidTransition;
use player::Player;
use rand::SeedableRng;

mod errors;
mod player;

/// The Game
#[derive(Debug, Clone)]
pub struct Judgment {
    stage: Stage,
    players: Vec<Player>,
    trick: Trick,
    scores: Vec<i64>,
    decks: u8,
    player_count: u8,
    starting_hand_size: u8,
    history: Vec<Transition>,
}

pub type Trick = Vec<Option<Card>>;

impl Judgment {
    /// Create a new game of Judgment for `players` and first round having
    /// `starting_hand_size` cards per player. Optionally, also specify how many
    /// card decks should be used.
    ///
    /// # Panics
    /// - Panics if the specified number of decks are insufficient to deal enough
    /// cards for the first round.
    /// - Panics if the `starting_hand_size` is larger than 13.
    ///
    /// # Examples
    /// ```
    /// use judgment::Judgment;
    ///
    /// Judgment::new(4, 13, None);
    /// ```
    /// ```should_panic
    /// use judgment::Judgment;
    ///
    /// Judgment::new(5, 13, Some(1));
    /// ```
    pub fn new(players: u8, starting_hand_size: u8, decks: Option<u8>) -> Self {
        assert!(
            starting_hand_size <= 13,
            "cannot start with more than 13 cards per player"
        );
        // SAFETY
        // cannot overflow because 13 * u8::MAX / 52 + 1 == 64
        let estimated_decks = starting_hand_size * players / 52
            + u8::from((u16::from(starting_hand_size) * u16::from(players)) % 52 != 0);
        let actual_decks = if let Some(decks) = decks {
            assert!(estimated_decks <= decks, "{decks} decks are not enough for the give number of players and starting hand size");
            decks
        } else {
            estimated_decks
        };
        Judgment {
            stage: Stage::PrePlay,
            players: vec![Player::new(); usize::from(players)],
            trick: vec![None; usize::from(players)],
            scores: vec![0; usize::from(players)],
            decks: actual_decks,
            player_count: players,
            starting_hand_size,
            history: Vec::new(),
        }
    }

    /// Try to advance the game with the `transition`.
    pub fn update(&mut self, transition: Transition) -> Result<(), InvalidTransition> {
        let res = match (&mut self.stage, transition) {
            (Stage::PrePlay, Transition::Deal { .. }) => {
                Err(InvalidTransition::DealBeforeGameStart)
            }
            (Stage::PrePlay, Transition::Play { .. }) => {
                Err(InvalidTransition::PlayBeforeScorePrediction)
            }
            (Stage::PrePlay, Transition::PredictScore { .. }) => {
                Err(InvalidTransition::PredictBeforeDeal)
            }
            (Stage::Deal(round), Transition::Deal { seed }) => {
                let hand_size = round.hand_size;
                self.stage = Stage::PredictScores(round.clone());
                self.deal(hand_size, seed);
                Ok(())
            }
            (Stage::Deal(_), Transition::Play { .. }) => {
                Err(InvalidTransition::PlayBeforeScorePrediction)
            }
            (Stage::Deal(_), Transition::PredictScore { .. }) => {
                Err(InvalidTransition::PredictBeforeDeal)
            }
            (Stage::PredictScores(_), Transition::Deal { .. }) => Err(InvalidTransition::ReDeal),
            (Stage::PredictScores(_), Transition::Play { .. }) => {
                Err(InvalidTransition::PlayBeforeScorePrediction)
            }
            (Stage::PredictScores(round), Transition::PredictScore { player, score }) => {
                if round.player != player {
                    return Err(InvalidTransition::OutOfTurnPlay);
                }
                if score > round.hand_size {
                    return Err(InvalidTransition::PredictionOutOfRange);
                }
                if round
                    .predicted_scores
                    .iter()
                    .filter(|score| score.is_some())
                    .count()
                    == usize::from(self.player_count - 1)
                {
                    let prediction_sum = round
                        .predicted_scores
                        .iter()
                        .filter_map(|opt_v| opt_v.map(u16::from))
                        .sum::<u16>();
                    if prediction_sum + u16::from(score) == u16::from(round.hand_size) {
                        return Err(InvalidTransition::LastPlayerPrediction);
                    }
                }
                round.predicted_scores[player] = Some(score);
                // SAFETY
                // `as` conversion is fine because `Round { player }` < `Judgment { player_count: u8 }`
                round.player = usize::from((round.player as u8 + 1) % self.player_count);
                if round
                    .predicted_scores
                    .iter()
                    .filter(|score| score.is_some())
                    .count()
                    == usize::from(self.player_count)
                {
                    self.stage = Stage::Play(round.clone());
                }
                Ok(())
            }
            (Stage::Play(_), Transition::Deal { .. }) => Err(InvalidTransition::ReDeal),
            (Stage::Play(round), Transition::Play { player, card }) => {
                if round.player != player {
                    return Err(InvalidTransition::OutOfTurnPlay);
                }
                if self.players[player].remove(&card).is_none() {
                    return Err(InvalidTransition::NoSuchPlayerCard);
                }
                self.trick[player] = Some(card);
                // SAFETY
                // `as` conversion is fine because `Round { player }` < `Judgment { player_count: u8 }`
                round.player = usize::from((round.player as u8 + 1) % self.player_count);
                // The play ends ^ here. The rest is for updating the state.
                // check whether the winning player should be updated.
                if trick_card_comparator(
                    &self.trick[round.potential_winner].unwrap(),
                    &card,
                    round.trump_suit.as_ref(),
                )
                .is_gt()
                {
                    round.potential_winner = player;
                }
                // check whether current trick turn is complete.
                if self.trick.len() == self.player_count.into() {
                    round.trick_scores[round.potential_winner] += 1;
                    round.player = round.potential_winner;
                    self.trick.clear();
                    // check whether the whole round is over.
                    if self.players[0].hand().is_empty() {
                        for (idx, score) in round.trick_scores.iter().enumerate() {
                            if round.predicted_scores[idx]
                                .is_some_and(|prediction| prediction == *score)
                            {
                                self.scores[idx] += i64::from(*score);
                            } else {
                                self.scores[idx] -= i64::from(*score);
                            }
                        }
                        if round.hand_size == 1 {
                            self.stage = Stage::Over;
                        } else {
                            self.stage = Stage::Deal(Round {
                                player: round.player,
                                potential_winner: round.player,
                                hand_size: round.hand_size - 1,
                                trump_suit: match round.trump_suit {
                                    Some(suit) => match suit {
                                        Suit::Diamonds => None,
                                        Suit::Clubs => Some(Suit::Diamonds),
                                        Suit::Hearts => Some(Suit::Clubs),
                                        Suit::Spades => Some(Suit::Hearts),
                                    },
                                    None => Some(Suit::Spades),
                                },
                                predicted_scores: vec![None; usize::from(self.player_count)],
                                trick_scores: vec![0; self.player_count.into()],
                                starting_player: (round.starting_player + 1)
                                    % usize::from(self.player_count),
                            });
                        }
                    }
                }
                Ok(())
            }
            (Stage::Play(_), Transition::PredictScore { .. }) => Err(InvalidTransition::RePredict),
            (Stage::Over { .. }, _) => Err(InvalidTransition::GameOver),
        };
        if res.is_ok() {
            self.history.push(transition);
        }
        res
    }

    /// Try to start the game.
    ///
    /// Errors if the game is already in progress or finished.
    pub fn start(&mut self) -> Result<(), InvalidTransition> {
        if !matches!(self.stage, Stage::PrePlay) {
            return Err(InvalidTransition::Restart);
        }
        let round = Round {
            player: 0,
            potential_winner: 0,
            hand_size: self.starting_hand_size,
            trump_suit: Some(Suit::Spades),
            predicted_scores: vec![None; usize::from(self.player_count)],
            trick_scores: vec![0; self.player_count.into()],
            starting_player: 0,
        };
        self.stage = Stage::Deal(round);
        Ok(())
    }

    fn deal(&mut self, hand_size: u8, random_seed: [u8; 32]) {
        let mut deck = StandardDeckBuilder::new()
            .subdecks(self.decks.into())
            .build();
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(random_seed);
        deck.shuffle(&mut rng);
        for player in self.players.iter_mut() {
            player.assign(deck.draw_n(hand_size.into()));
        }
    }

    pub fn scores(&self) -> &[i64] {
        &self.scores
    }

    pub fn trick(&self) -> &Trick {
        &self.trick
    }

    pub fn hand_of_player(&self, player: usize) -> Option<&[Card]> {
        self.players.get(player).map(|player| player.hand())
    }

    pub fn is_over(&self) -> bool {
        matches!(self.stage, Stage::Over)
    }

    pub fn predicted_scores(&self) -> Option<&[Option<u8>]> {
        match &self.stage {
            Stage::PrePlay | Stage::Deal(_) | Stage::Over => None,
            Stage::PredictScores(Round {
                predicted_scores, ..
            }) => Some(predicted_scores),
            Stage::Play(Round {
                predicted_scores, ..
            }) => Some(predicted_scores),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Round {
    player: usize,
    potential_winner: usize,
    hand_size: u8,
    trump_suit: Option<Suit>,
    predicted_scores: Vec<Option<u8>>,
    trick_scores: Vec<u8>,
    starting_player: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Stage {
    PrePlay,
    Deal(Round),
    PredictScores(Round),
    Play(Round),
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Deal { seed: [u8; 32] },
    Play { player: usize, card: Card },
    PredictScore { player: usize, score: u8 },
}

/// [`Rank::Numeric(2)`] is the lowest and [`Rank::Ace`] is the highest. Suit
/// ordering has no gameplay significance; it is only meant to arbitrarily order
/// suits in alternating reds and blacks.
fn card_comparator(c1: &Card, c2: &Card) -> std::cmp::Ordering {
    match (c1.suit().unwrap(), c2.suit().unwrap()) {
        (s1, s2) if s1 == s2 => match (c1.rank().unwrap(), c2.rank().unwrap()) {
            (r1, r2) if r1 == r2 => std::cmp::Ordering::Equal,
            (Rank::Ace, _) => std::cmp::Ordering::Greater,
            (Rank::Jack, Rank::Queen | Rank::King | Rank::Ace) => std::cmp::Ordering::Less,
            (Rank::Jack, _) => std::cmp::Ordering::Greater,
            (Rank::Queen, Rank::King | Rank::Ace) => std::cmp::Ordering::Less,
            (Rank::Queen, _) => std::cmp::Ordering::Greater,
            (Rank::King, Rank::Ace) => std::cmp::Ordering::Less,
            (Rank::King, _) => std::cmp::Ordering::Greater,
            (Rank::Numeric(r1), Rank::Numeric(r2)) => r1.cmp(r2),
            (Rank::Numeric(_), _) => std::cmp::Ordering::Less,
        },
        (Suit::Clubs, _) => std::cmp::Ordering::Less,
        (Suit::Diamonds, Suit::Clubs) => std::cmp::Ordering::Greater,
        (Suit::Diamonds, _) => std::cmp::Ordering::Less,
        (Suit::Hearts, Suit::Spades) => std::cmp::Ordering::Less,
        (Suit::Hearts, _) => std::cmp::Ordering::Greater,
        (Suit::Spades, _) => std::cmp::Ordering::Greater,
    }
}

/// Within the same suit, [`Rank::Numeric(2)`] is the lowest and [`Rank::Ace`]
/// is the highest. Suits have no ordering except the trump suit, if any, is
/// better than other suits. If none of the previous conditions can resolve the
/// ordering, the first played card is better.
fn trick_card_comparator(
    first: &Card,
    second: &Card,
    trump_suit: Option<&Suit>,
) -> std::cmp::Ordering {
    match (first.suit().unwrap(), second.suit().unwrap()) {
        (s1, s2) if s1 == s2 => card_comparator(first, second),
        (_, s2) => {
            // s1 and s2 are not the same here
            if trump_suit.is_some_and(|suit| suit == s2) {
                // s2 is trump, so always better than `first`
                std::cmp::Ordering::Less
            } else {
                // either there is no trump, or s2 is not the trump
                // so first card played is better
                std::cmp::Ordering::Greater
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use card_deck::standard_deck::{Card, Rank, Suit};

    use crate::trick_card_comparator;

    #[test]
    fn test_trick_card_comparison_without_trump() {
        let card_pairs_comparisons = [
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Clubs, Rank::Numeric(10)),
                Ordering::Greater,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Numeric(10)),
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Ordering::Less,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Ordering::Equal,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Jack),
                Ordering::Greater,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Numeric(10)),
                Ordering::Greater,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Queen),
                Ordering::Greater,
            ),
        ];
        for (first, second, expected_comparison) in card_pairs_comparisons {
            assert_eq!(
                trick_card_comparator(&first, &second, None),
                expected_comparison,
                "comparison failed for {first} and {second}"
            );
        }
    }

    #[test]
    fn test_trick_card_comparison_with_trump() {
        let card_pairs_comparisons = [
            // trump same as cards
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Clubs, Rank::Numeric(10)),
                Some(Suit::Clubs),
                Ordering::Greater,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Numeric(10)),
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Some(Suit::Clubs),
                Ordering::Less,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Some(Suit::Clubs),
                Ordering::Equal,
            ),
            // trump distinct from cards
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Clubs, Rank::Numeric(10)),
                Some(Suit::Diamonds),
                Ordering::Greater,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Numeric(10)),
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Some(Suit::Diamonds),
                Ordering::Less,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Some(Suit::Diamonds),
                Ordering::Equal,
            ),
            // trump same as one of the cards
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Jack),
                Some(Suit::Diamonds),
                Ordering::Less,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Numeric(10)),
                Some(Suit::Diamonds),
                Ordering::Less,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Queen),
                Some(Suit::Diamonds),
                Ordering::Less,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Jack),
                Some(Suit::Clubs),
                Ordering::Greater,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Numeric(10)),
                Some(Suit::Clubs),
                Ordering::Greater,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Queen),
                Some(Suit::Clubs),
                Ordering::Greater,
            ),
            // trump distinct from cards
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Jack),
                Some(Suit::Hearts),
                Ordering::Greater,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Numeric(10)),
                Some(Suit::Hearts),
                Ordering::Greater,
            ),
            (
                Card::new_normal(Suit::Clubs, Rank::Jack),
                Card::new_normal(Suit::Diamonds, Rank::Queen),
                Some(Suit::Hearts),
                Ordering::Greater,
            ),
        ];
        for (first, second, trump, expected_comparison) in card_pairs_comparisons {
            assert_eq!(
                trick_card_comparator(&first, &second, trump.as_ref()),
                expected_comparison,
                "comparison failed for {first}, {second} and {trump:?}"
            );
        }
    }
}
