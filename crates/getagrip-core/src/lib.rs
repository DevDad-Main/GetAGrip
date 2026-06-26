//! GetAGrip — core foundation.
//!
//! This crate owns every primitive the rest of the workspace depends on and
//! **nothing else**. No UI, no database driver, no plugin system. Every other
//! crate is built on top of `getagrip-core`.
//!
//! ## Module layout
//!
//! * [`error`]     — [`AtlasError`] enum + [`AtlasResult`] alias, miette-ready.
//! * [`id`]        — strongly-typed UUID-backed identifiers for every entity.
//! * [`events`]    — lock-free [`EventBus`] for inter-subsystem communication.
//! * [`config`]    — [`WorkspaceConfig`] read via `figment`.
//! * [`secrets`]   — encrypted credentials vault backed by the OS keyring
//!                   when available, falling back to an encrypted file.
//! * [`session`]   — connection profiles, folders, favorites, tags.
//!
//! ## Coding conventions
//!
//! * Return `AtlasResult<T>` from every public function.
//! * Construct IDs via `Id::new()` — never hand-craft `Uuid`.
//! * Keep modules small and focused; each module is one concern.
//! * composition-over-inheritance: define behavior with traits, not towers of
//!   `impl A for B for C`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod events;
pub mod id;
pub mod secrets;
pub mod session;

// Ergonomic re-exports: consumers write `getagrip_core::AtlasError` instead of
// reaching into sub-modules.
pub use error::{err_msg, AtlasError, AtlasResult};
pub use events::{Event, EventBus, EventHandler, Subscription};
pub use id::{Id, IdTag, Idable};
pub use config::WorkspaceConfig;
pub use secrets::{Secret, SecretKind, SecretsVault};
pub use session::{
    ConnectionDriver, ConnectionProfile, ConnectionProfiles, Credential, CredentialStore,
    EnvironmentColor, SaveRule,
};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name.
pub const NAME: &str = env!("CARGO_PKG_NAME");
