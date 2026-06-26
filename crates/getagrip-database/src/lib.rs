//! Database driver abstraction layer for GetAGrip.
//!
//! This crate defines the [`DatabaseDriver`] trait that every database
//! driver must implement, plus a [`ConnectionPool`] for managing
//! concurrent access to a single data source.
//!
//! ## Architecture
//!
//! * [`driver`] — the [`DatabaseDriver`] trait and shared types.
//! * [`pool`] — async connection pool with health checks.
//! * [`manager`] — maps [`ConnectionProfile`]s to live connections.

pub mod driver;
pub mod manager;
pub mod pool;

pub use driver::{
    ColumnInfo, ColumnType, ConnectionInfo, DatabaseDriver, DriverCapability, QueryResult,
    ResultRow, ServerVersion, Value,
};
pub use manager::{ConnectionManager, ManagedConnection};
pub use pool::{ConnectionPool, PoolConfig, PoolError, PooledConnection};
