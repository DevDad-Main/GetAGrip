//! GetAGrip database layer.
//!
//! Provides connection management, driver registry, and database driver
//! implementations for all supported database engines.

pub mod connection;
pub mod drivers;
pub mod registry;

pub use connection::ConnectionManager;
pub use registry::DriverRegistry;
