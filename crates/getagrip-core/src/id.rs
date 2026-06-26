//! Strongly-typed UUID-backed identifiers.
//!
//! Every entity in AtlasDB Studio—connections, editor tabs, query sessions,
//! results, plugins, etc.—is identified by a typed [`Id<T>`]. This eliminates
//! confusion between e.g. a `ConnectionId` and a `QueryId`, both of which
//! would be `Uuid` in a naive design.

use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trait implemented by tag types created with [`id_tag!`].
pub trait IdTag: Sized {
    /// Human-readable entity kind, e.g. `"connection"`.
    fn kind() -> &'static str;
}

/// A strongly-typed identifier wrapping a [`Uuid`].
///
/// `T` is a zero-sized tag type that prevents accidental mixing of
/// different identifier kinds.
#[derive(Copy, Clone, Eq, PartialOrd, Ord)]
pub struct Id<T: IdTag> {
    inner: Uuid,
    _tag: PhantomData<fn() -> T>,
}

impl<T: IdTag> Id<T> {
    /// Generate a new random v4 identifier.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Uuid::new_v4(),
            _tag: PhantomData,
        }
    }

    /// Generate a time-sortable v7 identifier (preferred for databases).
    #[must_use]
    pub fn new_v7() -> Self {
        Self {
            inner: Uuid::now_v7(),
            _tag: PhantomData,
        }
    }

    /// Parse an [`Id`] from its string representation.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(|inner| Self {
            inner,
            _tag: PhantomData,
        })
    }

    /// Access the raw [`Uuid`].
    #[must_use]
    pub fn as_uuid(&self) -> &Uuid {
        &self.inner
    }

    /// Consume the [`Id`] and return the inner [`Uuid`].
    #[must_use]
    pub fn into_uuid(self) -> Uuid {
        self.inner
    }

    /// The entity kind string (e.g. `"connection"`).
    #[must_use]
    pub fn kind() -> &'static str {
        T::kind()
    }
}

impl<T: IdTag> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: IdTag> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&format!("{}Id", T::kind()))
            .field(&self.inner.to_string())
            .finish()
    }
}

impl<T: IdTag> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T: IdTag> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T: IdTag> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: IdTag> Serialize for Id<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.inner.to_string().serialize(s)
    }
}

impl<'de, T: IdTag> Deserialize<'de> for Id<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom("invalid UUID"))
    }
}

/// Trait for any type that exposes a stable identifier.
pub trait Idable {
    /// The type of [`Id`] this entity uses.
    type Tag: IdTag;

    /// Return the entity's identifier.
    fn id(&self) -> Id<Self::Tag>;
}

/// Declare a new [`Id`] tag type (a unit struct implementing [`IdTag`]).
#[macro_export]
macro_rules! id_tag {
    ($vis:vis $name:ident => $kind:literal) => {
        #[doc = concat!("Tag type for [`Id<", stringify!($name), ">`] (", $kind, ").")]
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        $vis struct $name;

        impl $crate::IdTag for $name {
            fn kind() -> &'static str {
                $kind
            }
        }
    };
}

/// Convenience macro: declare the tag **and** a type alias in one shot.
#[macro_export]
macro_rules! id_alias {
    ($vis:vis $name:ident => $kind:literal) => {
        $crate::id_tag! { $vis $name => $kind }
        #[doc = concat!("Strongly-typed identifier for ", $kind, " entities.")]
        $vis type $name = $crate::Id<$name>;
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Id;

    id_tag!(ConnectionId => "connection");
    id_tag!(TabId => "tab");

    #[test]
    fn new_generates_distinct() {
        let a = Id::<ConnectionId>::new();
        let b = Id::<ConnectionId>::new();
        assert_ne!(a, b);
    }

    #[test]
    fn v7_is_distinct() {
        let a = Id::<ConnectionId>::new_v7();
        let b = Id::<ConnectionId>::new_v7();
        assert!(a != b);
    }

    #[test]
    fn different_tags_compile() {
        let a = Id::<ConnectionId>::new();
        let b = Id::<TabId>::new();
        let _ = (a, b);
    }

    #[test]
    fn kind_is_correct() {
        assert_eq!(Id::<ConnectionId>::kind(), "connection");
        assert_eq!(Id::<TabId>::kind(), "tab");
    }

    #[test]
    fn display_and_parse_roundtrip() {
        let id = Id::<ConnectionId>::new();
        let s = id.to_string();
        let parsed = Id::<ConnectionId>::parse(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn parse_invalid_returns_none() {
        assert!(Id::<ConnectionId>::parse("not-a-uuid").is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let id = Id::<ConnectionId>::new();
        let json = serde_json::to_string(&id).unwrap();
        let back: Id<ConnectionId> = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn default_is_new() {
        let a = Id::<ConnectionId>::default();
        let b = Id::<ConnectionId>::default();
        assert_ne!(a, b);
    }

    #[test]
    fn hash_consistent() {
        use std::hash::Hasher;
        let id = Id::<ConnectionId>::new();
        let mut h1 = std::collections::hash_map::DefaultHasher::new();
        let mut h2 = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut h1);
        id.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }
}
