#![cfg_attr(not(feature = "std"), no_std)]

pub mod logic;
#[cfg(feature = "hardware")]
pub mod hardware;
