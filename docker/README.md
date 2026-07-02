# Docker Development Databases

Spin up local databases for development and load-testing.

## Quick Start

```bash
# Start all services
docker compose up -d

# Start just PostgreSQL (fastest for testing)
docker compose up -d postgres

# Check seed progress
docker compose logs -f postgres
```

## Databases

| Service   | Port | User  | Password | Database | Connection URL (from app) |
|-----------|------|-------|----------|----------|---------------------------|
| Postgres  | 5432 | admin | admin    | testdb   | `postgresql://admin:admin@localhost:5432/testdb` |
| MySQL     | 3306 | admin | admin    | testdb   | `mysql://admin:admin@localhost:3306/testdb` |
| Mongo     | 27017| admin | admin    | testdb   | `mongodb://admin:admin@localhost:27017/testdb` |
| Redis     | 6379 | —     | —        | —        | `redis://localhost:6379` |
| Adminer   | 8081 | —     | —        | —        | http://localhost:8081 |

## Seed Data

- `01_seed.sql` — schema + tiny sample (3 users, 5 products, 4 orders)
- `02_load_test.sql` — **large datasets** for performance testing:

| Table       | Rows     | Purpose                                 |
|-------------|----------|-----------------------------------------|
| users       | 100,000  | Standard relational                     |
| products    | 10,000   | Lookup table                            |
| orders      | 500,000  | Fact table with status enum             |
| order_items | 2,000,000| Transactional lines                     |
| events      | 1,000,000| JSONB/JSON log data (wide columns)      |
| metrics     | 1,000,000| Time-series numeric data (server stats) |

Total: **~4.6M rows** across 6 tables.

## Testing the Virtual Scrolling Grid

After seeding, run these queries in GetAGrip to test the new virtual-scrolling ResultGrid:

### 500k rows — Simple SELECT
```sql
SELECT * FROM orders ORDER BY id;
```

### 1M rows — JSON columns
```sql
SELECT * FROM events ORDER BY id;
```

### 1M rows — Many numeric columns
```sql
SELECT * FROM metrics ORDER BY id;
```

### 2M rows — Join-heavy (tests sort/filter perf)
```sql
SELECT o.id, u.name, p.name AS product, oi.quantity, oi.unit_price
FROM orders o
JOIN users u ON u.id = o.user_id
JOIN order_items oi ON oi.order_id = o.id
JOIN products p ON p.id = oi.product_id
ORDER BY o.id DESC;
```

### Full table scan with filter
```sql
SELECT * FROM events WHERE event_type = 'error' ORDER BY created_at DESC;
```

### Large aggregation
```sql
SELECT category, COUNT(*) AS cnt, ROUND(AVG(price), 2) AS avg_price
FROM products
GROUP BY category
ORDER BY cnt DESC;
```

## Reset Data

```bash
# Wipe and re-seed
docker compose down -v
docker compose up -d
```
