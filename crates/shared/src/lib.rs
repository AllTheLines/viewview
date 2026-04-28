//! Code we want to share amongst internal crates.

#![allow(
    clippy::exhaustive_structs,
    reason = "This lib is mostly for our internal use"
)]

pub mod metadata;
pub mod projector;
pub mod utils;
