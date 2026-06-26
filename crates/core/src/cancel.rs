//! Cancellation tokens for async operations.
//!
//! Every long-running operation in GetAGrip accepts a cancellation token
//! so the user can cancel queries, connections, and background tasks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

/// A token that can be used to signal cancellation of an async operation.
#[derive(Clone, Debug)]
pub struct CancellationToken {
    inner: Arc<CancellationTokenInner>,
}

#[derive(Debug)]
struct CancellationTokenInner {
    cancelled: AtomicBool,
    notify: Notify,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    /// Create a new, uncancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CancellationTokenInner {
                cancelled: AtomicBool::new(false),
                notify: Notify::new(),
            }),
        }
    }

    /// Cancel the token. Idempotent.
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        self.inner.notify.notify_waiters();
    }

    /// Returns `true` if the token has been cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Wait until the token is cancelled.
    pub async fn wait(&self) {
        let notified = self.inner.notify.notified();
        if !self.is_cancelled() {
            notified.await;
        }
    }
}

/// A source that produces cancellation tokens.
///
/// Use this when you need to create child tokens that all share
/// the same cancellation signal.
#[derive(Clone, Debug, Default)]
pub struct CancellationTokenSource {
    token: CancellationToken,
}

impl CancellationTokenSource {
    /// Create a new source.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Get a clone of the token.
    #[must_use]
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Cancel all tokens produced by this source.
    pub fn cancel(&self) {
        self.token.cancel();
    }
}
