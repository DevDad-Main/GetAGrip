-- Load test: generates large datasets for performance testing (MySQL 8+).
-- Run after 01_seed.sql.
-- Uses recursive CTEs; be patient, this takes ~30-60s.

SET SESSION cte_max_recursion_depth = 1000000;
SET SESSION group_concat_max_len = 1000000;

-- =============================================================================
-- 1. USERS — 100k rows
-- =============================================================================
INSERT INTO users (name, email)
WITH RECURSIVE nums AS (
    SELECT 1 AS n
    UNION ALL
    SELECT n + 1 FROM nums WHERE n < 100000
)
SELECT CONCAT('User_', n), CONCAT('user_', n, '@example.com')
FROM nums;

-- =============================================================================
-- 2. PRODUCTS — 10k rows
-- =============================================================================
INSERT INTO products (name, price, stock, category)
WITH RECURSIVE nums AS (
    SELECT 1 AS n
    UNION ALL
    SELECT n + 1 FROM nums WHERE n < 10000
)
SELECT
    CONCAT('Product_', n),
    ROUND(RAND() * 500 + 5, 2),
    FLOOR(RAND() * 1000),
    ELT(FLOOR(RAND() * 8 + 1), 'Electronics','Accessories','Furniture','Clothing','Books','Food','Tools','Sports')
FROM nums;

-- =============================================================================
-- 3. ORDERS — 500k rows
-- =============================================================================
INSERT INTO orders (user_id, total, status)
WITH RECURSIVE nums AS (
    SELECT 1 AS n
    UNION ALL
    SELECT n + 1 FROM nums WHERE n < 500000
)
SELECT
    FLOOR(RAND() * 100000 + 1),
    ROUND(RAND() * 2000 + 10, 2),
    ELT(FLOOR(RAND() * 5 + 1), 'pending','processing','shipped','delivered','cancelled')
FROM nums;

-- =============================================================================
-- 4. ORDER ITEMS — 2M rows
-- =============================================================================
INSERT INTO order_items (order_id, product_id, quantity, unit_price)
WITH RECURSIVE nums AS (
    SELECT 1 AS n
    UNION ALL
    SELECT n + 1 FROM nums WHERE n < 2000000
)
SELECT
    FLOOR(RAND() * 500000 + 1),
    FLOOR(RAND() * 10000 + 1),
    FLOOR(RAND() * 10 + 1),
    ROUND(RAND() * 500 + 5, 2)
FROM nums;

-- =============================================================================
-- 5. EVENTS — 1M rows  (new table — JSON log)
-- =============================================================================
CREATE TABLE IF NOT EXISTS events (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    event_type VARCHAR(50) NOT NULL DEFAULT 'pageview',
    user_id INT,
    payload JSON NOT NULL DEFAULT (JSON_OBJECT()),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO events (event_type, user_id, payload)
WITH RECURSIVE nums AS (
    SELECT 1 AS n
    UNION ALL
    SELECT n + 1 FROM nums WHERE n < 1000000
)
SELECT
    ELT(FLOOR(RAND() * 7 + 1), 'pageview','click','purchase','login','logout','error','search'),
    FLOOR(RAND() * 100000 + 1),
    JSON_OBJECT(
        'url', ELT(FLOOR(RAND() * 4 + 1), '/home', '/products', '/cart', '/account'),
        'duration_ms', FLOOR(RAND() * 30000),
        'browser', ELT(FLOOR(RAND() * 4 + 1), 'Chrome','Firefox','Safari','Edge'),
        'referrer', CASE WHEN RAND() > 0.5 THEN 'google' WHEN RAND() > 0.3 THEN 'direct' ELSE 'social' END
    )
FROM nums;

-- =============================================================================
-- 6. METRICS — 1M rows  (new table — time-series numeric data)
-- =============================================================================
CREATE TABLE IF NOT EXISTS metrics (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    server_id INT NOT NULL DEFAULT 1,
    cpu_pct DECIMAL(5,2) NOT NULL,
    mem_mb DECIMAL(10,2) NOT NULL,
    disk_io DECIMAL(12,2) NOT NULL DEFAULT 0,
    net_rx_bytes BIGINT NOT NULL DEFAULT 0,
    net_tx_bytes BIGINT NOT NULL DEFAULT 0,
    recorded_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO metrics (server_id, cpu_pct, mem_mb, disk_io, net_rx_bytes, net_tx_bytes)
WITH RECURSIVE nums AS (
    SELECT 1 AS n
    UNION ALL
    SELECT n + 1 FROM nums WHERE n < 1000000
)
SELECT
    FLOOR(RAND() * 20 + 1),
    ROUND(RAND() * 100, 2),
    ROUND(RAND() * 32768, 2),
    ROUND(RAND() * 1000, 2),
    FLOOR(RAND() * 1073741824),
    FLOOR(RAND() * 1073741824)
FROM nums;

-- =============================================================================
-- STATS
-- =============================================================================
SELECT 'users' AS tbl, COUNT(*) FROM users
UNION ALL SELECT 'products', COUNT(*) FROM products
UNION ALL SELECT 'orders', COUNT(*) FROM orders
UNION ALL SELECT 'order_items', COUNT(*) FROM order_items
UNION ALL SELECT 'events', COUNT(*) FROM events
UNION ALL SELECT 'metrics', COUNT(*) FROM metrics;
