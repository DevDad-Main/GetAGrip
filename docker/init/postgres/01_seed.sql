CREATE SCHEMA analytics;

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id),
    total DECIMAL(10,2) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    price DECIMAL(10,2) NOT NULL,
    stock INTEGER DEFAULT 0,
    category VARCHAR(100)
);

CREATE TABLE order_items (
    id SERIAL PRIMARY KEY,
    order_id INTEGER REFERENCES orders(id),
    product_id INTEGER REFERENCES products(id),
    quantity INTEGER NOT NULL,
    unit_price DECIMAL(10,2) NOT NULL
);

INSERT INTO users (name, email) VALUES
    ('Alice Johnson', 'alice@example.com'),
    ('Bob Smith', 'bob@example.com'),
    ('Carol White', 'carol@example.com');

INSERT INTO products (name, price, stock, category) VALUES
    ('Wireless Mouse', 29.99, 150, 'Electronics'),
    ('Mechanical Keyboard', 89.99, 75, 'Electronics'),
    ('USB-C Hub', 45.00, 200, 'Accessories'),
    ('27" Monitor', 349.99, 30, 'Electronics'),
    ('Standing Desk', 599.99, 15, 'Furniture');

INSERT INTO orders (user_id, total, status) VALUES
    (1, 119.98, 'shipped'),
    (2, 349.99, 'processing'),
    (3, 689.98, 'delivered'),
    (1, 45.00, 'pending');

INSERT INTO order_items (order_id, product_id, quantity, unit_price) VALUES
    (1, 1, 2, 29.99),
    (1, 3, 1, 45.00),
    (2, 4, 1, 349.99),
    (3, 5, 1, 599.99),
    (3, 2, 1, 89.99),
    (4, 3, 1, 45.00);
