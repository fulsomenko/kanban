use kanban_core::PaginatedList;
use serde::Serialize;

use crate::archived_card::{ArchivedCard, ArchivedCardSummary};
use crate::card::{Card, CardSummary};

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PaginatedCards {
    Summaries(PaginatedList<CardSummary>),
    Full(PaginatedList<Card>),
}

impl PaginatedCards {
    pub fn new(cards: Vec<Card>, include_description: bool, page: usize, page_size: usize) -> Self {
        if include_description {
            Self::Full(PaginatedList::paginate(cards, page, page_size))
        } else {
            let summaries = cards.iter().map(CardSummary::from).collect();
            Self::Summaries(PaginatedList::paginate(summaries, page, page_size))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PaginatedArchivedCards {
    Summaries(PaginatedList<ArchivedCardSummary>),
    Full(PaginatedList<ArchivedCard>),
}

impl PaginatedArchivedCards {
    pub fn new(
        cards: Vec<ArchivedCard>,
        include_description: bool,
        page: usize,
        page_size: usize,
    ) -> Self {
        if include_description {
            Self::Full(PaginatedList::paginate(cards, page, page_size))
        } else {
            let summaries = cards.iter().map(ArchivedCardSummary::from).collect();
            Self::Summaries(PaginatedList::paginate(summaries, page, page_size))
        }
    }
}
