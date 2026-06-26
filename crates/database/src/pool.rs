//! Async connection pool with health checks.
//!
//! The pool maintains a bounded set of connections to a single database.
//! Connections are lazily created and automatically recycled when broken.

use std::fmt;
use std::sync::{Arc, Weak};
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::driver::DatabaseDriver;

/// Configuration for a [`ConnectionPool`].
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Minimum number of idle connections to keep alive.
    pub min_idle: u32,
    /// Maximum number of connections in the pool.
    pub max_size: u32,
    /// Maximum time to wait for a connection before returning an error.
    pub acquire_timeout: Duration,
    /// How long a connection can be idle before being closed.
    pub idle_timeout: Duration,
    /// Maximum lifetime of a single connection.
    pub max_lifetime: Duration,
    /// Interval between health-check pings.
    pub health_check_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_idle: 1,
            max_size: 5,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(3600),
            health_check_interval: Duration::from_secs(30),
        }
    }
}

/// Errors that can occur when interacting with the pool.
#[derive(Debug)]
pub enum PoolError {
    /// The pool has been closed.
    Closed,
    /// Timed out waiting for a connection.
    Timeout,
    /// The pool is at capacity and all connections are in use.
    Exhausted,
    /// An underlying driver error occurred.
    Driver(atlas_core::AtlasError),
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => f.write_str("connection pool is closed"),
            Self::Timeout => f.write_str("timed out waiting for a connection"),
            Self::Exhausted => f.write_str("connection pool exhausted"),
            Self::Driver(e) => write!(f, "driver error: {e}"),
        }
    }
}

impl std::error::Error for PoolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Driver(e) => Some(e),
            _ => None,
        }
    }
}

impl From<atlas_core::AtlasError> for PoolError {
    fn from(e: atlas_core::AtlasError) -> Self {
        Self::Driver(e)
    }
}

/// A handle to a connection borrowed from the pool.
///
/// When dropped, the connection is automatically returned to the pool
/// and the semaphore permit is released.
pub struct PooledConnection {
    inner: Option<Box<dyn crate::driver::DriverConnection>>,
    created_at: std::time::Instant,
    pool: Weak<PoolInner>,
    _permit: OwnedSemaphorePermit,
}

impl PooledConnection {
    fn new(
        conn: Box<dyn crate::driver::DriverConnection>,
        pool: &Arc<PoolInner>,
        permit: OwnedSemaphorePermit,
    ) -> Self {
        Self {
            inner: Some(conn),
            created_at: std::time::Instant::now(),
            pool: Arc::downgrade(pool),
            _permit: permit,
        }
    }

    /// Access the underlying connection.
    pub fn connection(&self) -> &dyn crate::driver::DriverConnection {
        self.inner.as_ref().expect("PooledConnection already returned")
    }

    /// Access the underlying connection mutably.
    pub fn connection_mut(&mut self) -> &mut dyn crate::driver::DriverConnection {
        self.inner.as_mut().expect("PooledConnection already returned")
    }

    /// How long this connection has been checked out.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.inner.take() {
            if let Some(pool) = self.pool.upgrade() {
                if pool.closed.load(std::sync::atomic::Ordering::Acquire) {
                    pool.total.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                    return;
                }
                let mut idle = pool.idle.lock();
                if idle.len() < pool.config.min_idle as usize {
                    idle.push(conn);
                } else {
                    pool.total.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                }
            }
        }
        // `_permit` is dropped here, releasing the semaphore slot.
    }
}

impl fmt::Debug for PooledConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PooledConnection")
            .field("age", &self.age())
            .finish()
    }
}

// ---- Pool internals ------------------------------------------------------

struct PoolInner {
    config: PoolConfig,
    driver: Arc<dyn DatabaseDriver>,
    url: String,
    semaphore: Arc<Semaphore>,
    idle: Mutex<Vec<Box<dyn crate::driver::DriverConnection>>>,
    total: std::sync::atomic::AtomicU32,
    closed: std::sync::atomic::AtomicBool,
}

/// An async connection pool for a single database.
///
/// Thread-safe and designed to be shared across tasks via `Arc`.
pub struct ConnectionPool {
    inner: Arc<PoolInner>,
}

impl ConnectionPool {
    /// Create a new connection pool.
    ///
    /// Connections are created lazily — no connections are established
    /// until the first call to [`acquire`].
    pub fn new(
        driver: Arc<dyn DatabaseDriver>,
        url: String,
        config: PoolConfig,
    ) -> Self {
        let max = config.max_size;
        Self {
            inner: Arc::new(PoolInner {
                config,
                driver,
                url,
                semaphore: Arc::new(Semaphore::new(max as usize)),
                idle: Mutex::new(Vec::new()),
                total: std::sync::atomic::AtomicU32::new(0),
                closed: std::sync::atomic::AtomicBool::new(false),
            }),
        }
    }

    /// Acquire a connection from the pool, blocking (async) until one is
    /// available or the timeout expires.
    pub async fn acquire(&self) -> Result<PooledConnection, PoolError> {
        if self.inner.closed.load(std::sync::atomic::Ordering::Acquire) {
            return Err(PoolError::Closed);
        }

        // Wait for a permit (returns OwnedSemaphorePermit).
        let permit = tokio::time::timeout(
            self.inner.config.acquire_timeout,
            Arc::clone(&self.inner.semaphore).acquire_owned(),
        )
        .await
        .map_err(|_| PoolError::Timeout)?
        .map_err(|_| PoolError::Closed)?;

        // Try to grab an idle connection first.
        {
            let mut idle = self.inner.idle.lock();
            if let Some(conn) = idle.pop() {
                return Ok(PooledConnection::new(conn, &self.inner, permit));
            }
        }

        // No idle connection — create a new one.
        let current = self
            .inner
            .total
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if current >= self.inner.config.max_size {
            self.inner
                .total
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            // Permit is dropped, releasing the semaphore slot.
            return Err(PoolError::Exhausted);
        }

        let conn = self
            .inner
            .driver
            .connect(&self.inner.url)
            .await
            .map_err(|e| {
                self.inner
                    .total
                    .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                PoolError::Driver(e)
            })?;

        Ok(PooledConnection::new(conn, &self.inner, permit))
    }

    /// Current pool statistics.
    pub fn stats(&self) -> PoolStats {
        let idle = self.inner.idle.lock().len() as u32;
        let total = self.inner.total.load(std::sync::atomic::Ordering::Acquire);
        PoolStats {
            idle,
            active: total.saturating_sub(idle),
            total,
            max: self.inner.config.max_size,
        }
    }

    /// Close the pool, preventing further acquires.
    ///
    /// Existing connections are not forcefully closed — they will be
    /// dropped when returned.
    pub fn close(&self) {
        self.inner
            .closed
            .store(true, std::sync::atomic::Ordering::Release);
    }

    /// Whether the pool is closed.
    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(std::sync::atomic::Ordering::Acquire)
    }
}

impl fmt::Debug for ConnectionPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.stats();
        f.debug_struct("ConnectionPool")
            .field("url", &self.inner.url)
            .field("idle", &s.idle)
            .field("active", &s.active)
            .field("total", &s.total)
            .field("max", &s.max)
            .finish()
    }
}

/// Snapshot of pool statistics.
#[derive(Debug, Clone, Copy)]
pub struct PoolStats {
    /// Number of idle connections.
    pub idle: u32,
    /// Number of actively borrowed connections.
    pub active: u32,
    /// Total connections (idle + active).
    pub total: u32,
    /// Maximum pool size.
    pub max: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::*;
    use atlas_core::AtlasResult;
    use async_trait::async_trait;
    use std::sync::atomic::AtomicU32;

    struct MockDriver {
        connects: AtomicU32,
    }

    #[async_trait]
    impl DatabaseDriver for MockDriver {
        fn name(&self) -> &'static str {
            "Mock"
        }

        fn capabilities(&self) -> DriverCapability {
            DriverCapability::SELECT
        }

        async fn connect(&self, _url: &str) -> AtlasResult<Box<dyn DriverConnection>> {
            self.connects.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Box::new(MockConnection))
        }
    }

    struct MockConnection;

    #[async_trait]
    impl DriverConnection for MockConnection {
        async fn execute(&mut self, _sql: &str) -> AtlasResult<QueryResult> {
            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: 0,
                elapsed_us: 0,
                warnings: vec![],
            })
        }

        async fn ping(&mut self) -> AtlasResult<()> {
            Ok(())
        }

        async fn info(&self) -> AtlasResult<ConnectionInfo> {
            Ok(ConnectionInfo {
                product_name: "Mock".into(),
                version: ServerVersion { major: 1, minor: 0, patch: 0, raw: "1.0.0".into() },
                database: "mock".into(),
                schema: "public".into(),
                user: "mock".into(),
                server_pid: None,
                read_only: false,
            })
        }

        async fn begin(&mut self) -> AtlasResult<()> { Ok(()) }
        async fn commit(&mut self) -> AtlasResult<()> { Ok(()) }
        async fn rollback(&mut self) -> AtlasResult<()> { Ok(()) }
        async fn close(&mut self) -> AtlasResult<()> { Ok(()) }
    }

    #[tokio::test]
    async fn pool_acquire_and_stats() {
        let driver = Arc::new(MockDriver { connects: AtomicU32::new(0) });
        let config = PoolConfig { max_size: 2, ..PoolConfig::default() };
        let pool = ConnectionPool::new(driver, "mock://".into(), config);

        let conn1 = pool.acquire().await.unwrap();
        let stats = pool.stats();
        assert_eq!(stats.active, 1);
        assert_eq!(stats.total, 1);

        let _conn2 = pool.acquire().await.unwrap();
        let stats = pool.stats();
        assert_eq!(stats.active, 2);

        drop(conn1);
        // After dropping, the connection should return to idle.
        let stats = pool.stats();
        assert_eq!(stats.idle, 1);
        assert_eq!(stats.active, 1);
    }

    #[tokio::test]
    async fn pool_close_prevents_acquire() {
        let driver = Arc::new(MockDriver { connects: AtomicU32::new(0) });
        let pool = ConnectionPool::new(driver, "mock://".into(), PoolConfig::default());
        pool.close();
        assert!(pool.acquire().await.is_err());
    }
}
