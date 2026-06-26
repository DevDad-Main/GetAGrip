//! Core trait definitions.
//!
//! Every major subsystem in GetAGrip is defined as a trait in this module.
//! Implementations live in their respective crates.

pub mod driver;
pub mod connection;
pub mod theme;
pub mod plugin;
pub mod ai;
pub mod export;
