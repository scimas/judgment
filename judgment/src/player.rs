use card_deck::standard_deck::Card;

use crate::card_comparator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Player {
    hand: Vec<Card>,
}

impl Player {
    pub(crate) fn new() -> Self {
        Player { hand: Vec::new() }
    }

    pub(crate) fn assign<I>(&mut self, cards: I)
    where
        I: Iterator<Item = Card>,
    {
        self.hand = cards.collect();
        self.hand.sort_by(card_comparator);
    }

    pub(crate) fn hand(&self) -> &[Card] {
        &self.hand
    }

    pub(crate) fn has(&self, card: &Card) -> bool {
        self.hand
            .binary_search_by(|h_card| card_comparator(h_card, card))
            .is_ok()
    }
}
