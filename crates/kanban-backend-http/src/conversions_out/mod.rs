//! Outbound domain -> request-DTO conversions for the v1 mutation routes; no
//! in-crate caller until the `RemoteWrites` impl lands.
#![allow(dead_code)]

mod boards;
mod cards;
mod columns;

pub(crate) use boards::{create_board_request, update_board_request};
pub(crate) use cards::{create_card_request, update_card_request};
pub(crate) use columns::{create_column_request, update_column_request};
