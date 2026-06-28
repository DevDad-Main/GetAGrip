db = db.getSiblingDB('testdb');

db.createCollection('users');
db.createCollection('orders');
db.createCollection('products');

db.users.insertMany([
    { name: 'Alice Johnson', email: 'alice@example.com', created_at: new Date() },
    { name: 'Bob Smith', email: 'bob@example.com', created_at: new Date() },
    { name: 'Carol White', email: 'carol@example.com', created_at: new Date() }
]);

db.products.insertMany([
    { name: 'Wireless Mouse', price: 29.99, stock: 150, category: 'Electronics' },
    { name: 'Mechanical Keyboard', price: 89.99, stock: 75, category: 'Electronics' },
    { name: 'USB-C Hub', price: 45.00, stock: 200, category: 'Accessories' },
    { name: '27" Monitor', price: 349.99, stock: 30, category: 'Electronics' },
    { name: 'Standing Desk', price: 599.99, stock: 15, category: 'Furniture' }
]);

db.orders.insertMany([
    { user_id: 1, total: 119.98, status: 'shipped', items: [{ product_id: 1, qty: 2, price: 29.99 }, { product_id: 3, qty: 1, price: 45.00 }] },
    { user_id: 2, total: 349.99, status: 'processing', items: [{ product_id: 4, qty: 1, price: 349.99 }] },
    { user_id: 3, total: 689.98, status: 'delivered', items: [{ product_id: 5, qty: 1, price: 599.99 }, { product_id: 2, qty: 1, price: 89.99 }] },
    { user_id: 1, total: 45.00, status: 'pending', items: [{ product_id: 3, qty: 1, price: 45.00 }] }
]);

db.users.createIndex({ email: 1 }, { unique: true });
db.orders.createIndex({ user_id: 1 });
db.products.createIndex({ category: 1 });
