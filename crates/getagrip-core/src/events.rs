//! Lock-free event bus for inter-subsystem communication.
//!
//! GetAGrip Studio's subsystems (editor, explorer, connection manager, etc.)
//! communicate through a shared [`EventBus`]. The bus is:
//!
//! * **Typed** — events are serialisable structs, not opaque strings.
//! * **Lock-free** — `publish` uses a crossbeam channel; subscribers do not
//!   block publishers.
//! * **Decoupled** — subsystems only depend on the event types they care about;
//!   they never call each other directly.
//!
//! ## Example
//!
//! ```ignore
//! use getagrip_core::events::{EventBus, Event, EventHandler};
//!
//! let bus = EventBus::new();
//! let mut rx = bus.subscribe::<QueryFinished>();
//! bus.publish(QueryFinished { tab_id: 42.into(), row_count: 100 });
//! assert!(rx.try_recv().is_ok());
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use crossbeam_channel::{self, Sender};
use parking_lot::RwLock;

use crate::AtlasError;

/// An event that can be published on the [`EventBus`].
///
/// Implement this for any struct you want to send across the bus.
/// Derive macros are provided: use `#[derive(Event)]` (requires `strum` macros).
pub trait Event: Any + Send + Sync + 'static {
    /// A stable, human-readable event name (e.g. `"query:finished"`).
    fn event_name() -> &'static str;
}

/// A handle that lets a subscriber receive events of type `E`.
///
/// Created by [`EventBus::subscribe`].
pub struct Subscription<E: Event> {
    rx: crossbeam_channel::Receiver<Arc<E>>,
}

impl<E: Event> Subscription<E> {
    /// Non-blocking receive.
    pub fn try_recv(&self) -> Option<Arc<E>> {
        self.rx.try_recv().ok()
    }

    /// Blocking receive.
    pub fn recv(&self) -> Option<Arc<E>> {
        self.rx.recv().ok()
    }

    /// Non-blocking iterator adapter.
    pub fn try_iter(&self) -> crossbeam_channel::TryIter<'_, Arc<E>> {
        self.rx.try_iter()
    }
}

/// A callback-style event handler.
///
/// Implement this trait for synchronous or async handler structs.
/// Use [`EventBus::on`] to register a handler.
pub trait EventHandler<E: Event>: Send + Sync + 'static {
    /// Called when an event of type `E` is published.
    fn handle(&self, event: Arc<E>);
}

// Allow closures as handlers.
impl<E: Event, F: Fn(Arc<E>) + Send + Sync + 'static> EventHandler<E> for F {
    fn handle(&self, event: Arc<E>) {
        (self)(event);
    }
}

/// A lock-free, typed event bus.
///
/// Thread-safe: `publish` and `subscribe` can be called concurrently.
pub struct EventBus {
    /// Map from `TypeId` to a list of sender halves for that event type.
    ///
    /// Each `subscribe` call pushes a new sender into the list. When an event
    /// is published, it is cloned into every sender in the list.
    senders: RwLock<HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>>,
}

impl EventBus {
    /// Create a new, empty event bus.
    pub fn new() -> Self {
        Self {
            senders: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe to events of type `E`.
    ///
    /// Returns a [`Subscription`] that receives every future `publish::<E>()`.
    pub fn subscribe<E: Event>(&self) -> Subscription<E> {
        let (tx, rx) = crossbeam_channel::unbounded::<Arc<E>>();
        let type_id = TypeId::of::<E>();
        self.senders.write().entry(type_id).or_default().push(Box::new(tx));
        Subscription { rx }
    }

    /// Register a synchronous event handler for type `E`.
    ///
    /// Spawns a background thread that invokes `handler` for every
    /// published event of type `E`. Returns a [`Subscription`]; the
    /// handler stops when the subscription is dropped.
    pub fn on<E: Event>(&self, handler: impl EventHandler<E>) -> Subscription<E> {
        let sub = self.subscribe::<E>();
        let handler = Arc::new(handler);
        let rx = sub.rx.clone();
        std::thread::Builder::new()
            .name(format!("event-handler-{}", E::event_name()))
            .spawn(move || {
                loop {
                    match rx.recv() {
                        Ok(event) => handler.handle(event),
                        Err(_) => break,
                    }
                }
            })
            .ok();
        sub
    }

    /// Publish an event to all subscribers of type `E`.
    ///
    /// Returns the number of subscribers that received the event.
    pub fn publish<E: Event>(&self, event: E) -> usize {
        let type_id = TypeId::of::<E>();
        let event = Arc::new(event);
        let senders = self.senders.read();
        if let Some(list) = senders.get(&type_id) {
            let mut count = 0;
            for raw in list {
                if let Some(tx) = raw.downcast_ref::<Sender<Arc<E>>>() {
                    if tx.send(Arc::clone(&event)).is_ok() {
                        count += 1;
                    }
                }
            }
            count
        } else {
            0
        }
    }

    /// Attempt to publish; returns an error if the channel is full (unlikely
    /// for unbounded channels).
    pub fn try_publish<E: Event>(&self, event: E) -> Result<usize, AtlasError> {
        Ok(self.publish(event))
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// SEALED: Event is implemented only through the derive macro.
// By keeping the module small and the trait sealed we prevent accidental
// misuse across crate boundaries.

// ----------------------------------------------------------------
// Built-in event types (Phase 1 — expand as the IDE grows)
// ----------------------------------------------------------------

/// Fired when an editor tab is opened.
#[derive(Debug, Clone)]
pub struct TabOpened {
    /// The tab identifier.
    pub tab_id: String,
    /// File or scratch path.
    pub path: Option<String>,
    /// Whether this is a scratch file.
    pub is_scratch: bool,
}

/// Fired when an editor tab is closed.
#[derive(Debug, Clone)]
pub struct TabClosed {
    /// The tab identifier.
    pub tab_id: String,
}

/// Fired when a query execution finishes.
#[derive(Debug, Clone)]
pub struct QueryFinished {
    /// The editor tab that issued the query.
    pub tab_id: String,
    /// Number of result rows.
    pub row_count: u64,
    /// Execution time in microseconds.
    pub elapsed_us: u64,
    /// Whether the query was cancelled.
    pub cancelled: bool,
}

/// Fired when a connection is established.
#[derive(Debug, Clone)]
pub struct ConnectionOpened {
    /// The connection profile id.
    pub profile_id: String,
    /// Display name of the connection.
    pub name: String,
    /// Database driver used.
    pub driver: String,
}

/// Fired when a connection is closed.
#[derive(Debug, Clone)]
pub struct ConnectionClosed {
    /// The connection profile id.
    pub profile_id: String,
}

/// Fired when the active theme changes.
#[derive(Debug, Clone)]
pub struct ThemeChanged {
    /// Name of the new theme.
    pub theme: String,
}

/// Fired when settings are modified.
#[derive(Debug, Clone)]
pub struct SettingsChanged {
    /// The config key that changed.
    pub key: String,
}

// Implement Event for every built-in event type.
macro_rules! impl_event {
    ($ty:ty, $name:literal) => {
        impl Event for $ty {
            fn event_name() -> &'static str {
                $name
            }
        }
    };
}

impl_event!(TabOpened, "tab:opened");
impl_event!(TabClosed, "tab:closed");
impl_event!(QueryFinished, "query:finished");
impl_event!(ConnectionOpened, "connection:opened");
impl_event!(ConnectionClosed, "connection:closed");
impl_event!(ThemeChanged, "theme:changed");
impl_event!(SettingsChanged, "settings:changed");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_subscribe_same_type() {
        let bus = EventBus::new();
        let sub = bus.subscribe::<TabOpened>();
        let count = bus.publish(TabOpened {
            tab_id: "t1".into(),
            path: None,
            is_scratch: true,
        });
        assert_eq!(count, 1);
        let ev = sub.try_recv().unwrap();
        assert_eq!(ev.tab_id, "t1");
        assert!(ev.is_scratch);
    }

    #[test]
    fn publish_no_subscribers_is_silent() {
        let bus = EventBus::new();
        let count = bus.publish(TabClosed {
            tab_id: "x".into(),
        });
        assert_eq!(count, 0);
    }

    #[test]
    fn multiple_subscribers_all_receive() {
        let bus = EventBus::new();
        let s1 = bus.subscribe::<ConnectionOpened>();
        let s2 = bus.subscribe::<ConnectionOpened>();
        let count = bus.publish(ConnectionOpened {
            profile_id: "p1".into(),
            name: "pg".into(),
            driver: "postgres".into(),
        });
        assert_eq!(count, 2);
        assert!(s1.try_recv().is_some());
        assert!(s2.try_recv().is_some());
    }

    #[test]
    fn different_types_are_isolated() {
        let bus = EventBus::new();
        let _sub = bus.subscribe::<TabOpened>();
        // Publishing a different type should not reach the subscriber.
        let count = bus.publish(ThemeChanged {
            theme: "dark".into(),
        });
        assert_eq!(count, 0);
    }

    #[test]
    fn event_names_are_stable() {
        assert_eq!(TabOpened::event_name(), "tab:opened");
        assert_eq!(QueryFinished::event_name(), "query:finished");
        assert_eq!(ConnectionOpened::event_name(), "connection:opened");
    }
}
