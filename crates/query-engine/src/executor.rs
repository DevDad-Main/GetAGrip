//! Query executor — orchestrates query execution with timing and cancellation.

use std::time::Instant;
use tg_core::cancel::CancellationToken;
use tg_core::error::{CoreError, CoreResult};
use tg_core::result::QueryResult;
use tg_core::traits::driver::Connection;
use tg_core::types::query::{Pagination, QueryParams};
use tracing::{debug, error, info, instrument};

/// Execute a query with instrumentation and cancellation support.
#[instrument(skip(conn, cancel), fields(sql_len = sql.len()))]
pub async fn execute(
    conn: &dyn Connection,
    sql: &str,
    params: Option<QueryParams>,
    pagination: Option<Pagination>,
    cancel: CancellationToken,
) -> CoreResult<QueryResult> {
    let start = Instant::now();

    // Check cancellation before starting
    if cancel.is_cancelled() {
        return Err(CoreError::Cancelled);
    }

    info!("Executing query");

    match conn.execute(sql, params, pagination, cancel).await {
        Ok(mut result) => {
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            debug!(
                rows = result.rows.len(),
                elapsed_ms = result.elapsed_ms,
                "Query completed"
            );
            Ok(result)
        }
        Err(e) => {
            error!(error = %e, elapsed_ms = start.elapsed().as_millis() as u64, "Query failed");
            Err(e)
        }
    }
}
