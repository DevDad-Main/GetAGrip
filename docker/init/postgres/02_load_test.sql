-- Load test: generates large datasets for performance testing.
-- Run after 01_seed.sql (tables must already exist).
-- The existing seed data stays; this appends to it.
--
-- Target row counts:
--   users:       100,000
--   products:     10,000
--   orders:      500,000
--   order_items: 2,000,000
--   events:    1,000,000  (new — JSONB log table)
--   metrics:   1,000,000  (new — time-series numeric table)

\timing on

-- =============================================================================
-- 1. USERS — 100k rows
-- =============================================================================
INSERT INTO users (name, email)
SELECT
    'User_' || n,
    'user_' || n || '@example.com'
FROM generate_series(4, 100000) AS n;

-- =============================================================================
-- 2. PRODUCTS — 10k rows
-- =============================================================================
INSERT INTO products (name, price, stock, category)
SELECT
    'Product_' || n,
    round((random() * 500 + 5)::numeric, 2),
    floor(random() * 1000)::int,
    (ARRAY['Electronics','Accessories','Furniture','Clothing','Books','Food','Tools','Sports'])[floor(random() * 8 + 1)]::varchar
FROM generate_series(6, 10000) AS n;

-- =============================================================================
-- 3. ORDERS — 500k rows
-- =============================================================================
INSERT INTO orders (user_id, total, status)
SELECT
    floor(random() * 100000 + 1)::int,
    round((random() * 2000 + 10)::numeric, 2),
    (ARRAY['pending','processing','shipped','delivered','cancelled'])[floor(random() * 5 + 1)]::varchar
FROM generate_series(5, 500004) AS n;

-- =============================================================================
-- 4. ORDER ITEMS — 2M rows (approx 4 per order)
-- =============================================================================
INSERT INTO order_items (order_id, product_id, quantity, unit_price)
SELECT
    floor(random() * 500000 + 1)::int,
    floor(random() * 10000 + 1)::int,
    floor(random() * 10 + 1)::int,
    round((random() * 500 + 5)::numeric, 2)
FROM generate_series(1, 2000000) AS n;

-- =============================================================================
-- 5. EVENTS — 1M rows  (new table — free-form JSONB payload)
-- =============================================================================
CREATE TABLE IF NOT EXISTS events (
    id BIGSERIAL PRIMARY KEY,
    event_type VARCHAR(50) NOT NULL DEFAULT 'pageview',
    user_id INT,
    payload JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO events (event_type, user_id, payload)
SELECT
    (ARRAY['pageview','click','purchase','login','logout','error','search'])[floor(random() * 7 + 1)],
    floor(random() * 100000 + 1)::int,
    jsonb_build_object(
        'url', CASE floor(random() * 4)::int
            WHEN 0 THEN '/home'
            WHEN 1 THEN '/products'
            WHEN 2 THEN '/cart'
            ELSE '/account'
        END,
        'duration_ms', floor(random() * 30000)::int,
        'browser', (ARRAY['Chrome','Firefox','Safari','Edge'])[floor(random() * 4 + 1)],
        'referrer', CASE WHEN random() > 0.5 THEN 'google' WHEN random() > 0.3 THEN 'direct' ELSE 'social' END
    )
FROM generate_series(1, 1000000) AS n;

-- =============================================================================
-- 6. METRICS — 1M rows  (new table — time-series numeric data)
-- =============================================================================
CREATE TABLE IF NOT EXISTS metrics (
    id BIGSERIAL PRIMARY KEY,
    server_id INT NOT NULL DEFAULT 1,
    cpu_pct NUMERIC(5,2) NOT NULL,
    mem_mb NUMERIC(10,2) NOT NULL,
    disk_io NUMERIC(12,2) NOT NULL DEFAULT 0,
    net_rx_bytes BIGINT NOT NULL DEFAULT 0,
    net_tx_bytes BIGINT NOT NULL DEFAULT 0,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO metrics (server_id, cpu_pct, mem_mb, disk_io, net_rx_bytes, net_tx_bytes)
SELECT
    floor(random() * 20 + 1)::int,
    round((random() * 100)::numeric, 2),
    round((random() * 32768)::numeric, 2),
    round((random() * 1000)::numeric, 2),
    floor(random() * 1073741824)::bigint,
    floor(random() * 1073741824)::bigint
FROM generate_series(1, 1000000) AS n;

-- =============================================================================
-- STATS
-- =============================================================================
SELECT 'users' AS tbl, count(*) FROM users
UNION ALL SELECT 'products', count(*) FROM products
UNION ALL SELECT 'orders', count(*) FROM orders
UNION ALL SELECT 'order_items', count(*) FROM order_items
UNION ALL SELECT 'events', count(*) FROM events
UNION ALL SELECT 'metrics', count(*) FROM metrics;
