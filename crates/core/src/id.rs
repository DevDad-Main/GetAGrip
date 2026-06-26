//! Strongly-typed identifiers for GetAGrip.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use uuid::Uuid;

/// A strongly-typed identifier.
///
/// The `T` parameter is a zero-sized tag type that prevents
/// accidental mixing of different kinds of IDs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<T> {
    inner: Uuid,
    #[serde(skip)]
    _marker: PhantomData<T>,
}

impl<T> Id<T> {
    /// Generate a new random ID.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Uuid::new_v4(),
            _marker: PhantomData,
        }
    }

    /// Create an ID from a raw UUID.
    #[must_use]
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self {
            inner: uuid,
            _marker: PhantomData,
        }
    }

    /// Get the underlying UUID.
    #[must_use]
    pub fn as_uuid(&self) -> &Uuid {
        &self.inner
    }

    /// Create from a string representation.
    ///
    /// # Errors
    /// Returns an error if the string is not a valid UUID.
    pub fn parse_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self {
            inner: Uuid::parse_str(s)?,
            _marker: PhantomData,
        })
    }
}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.inner)
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<T> FromStr for Id<T> {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

/// Tag types for different ID kinds.

/// Identifies a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionTag {}

/// Identifies a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryTag {}

/// Identifies a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabTag {}

/// Identifies a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceTag {}

/// Identifies a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginTag {}

/// Identifies a theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeTag {}

/// Identifies a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionTag {}

/// Identifies a driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DriverTag {}
