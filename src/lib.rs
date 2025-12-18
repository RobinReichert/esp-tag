#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "hardware")]
pub mod hardware;
pub mod logic;
