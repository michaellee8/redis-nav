#!/bin/bash

REDIS_CLI="podman exec -i redis-nav_redis_1 redis-cli"

echo "Seeding Redis with sample data..."

# Users
$REDIS_CLI SET "user:1:profile" '{"name": "Alice", "email": "alice@example.com", "theme": "dark"}'
$REDIS_CLI SET "user:1:settings" '{"notifications": true, "language": "en"}'
$REDIS_CLI SET "user:2:profile" '{"name": "Bob", "email": "bob@example.com", "theme": "light"}'
$REDIS_CLI EXPIRE "user:2:profile" 3600

# Cache entries
$REDIS_CLI SET "cache:page:home" "<html><body>Home Page</body></html>"
$REDIS_CLI SET "cache:page:about" "<html><body>About Page</body></html>"
$REDIS_CLI EXPIRE "cache:page:home" 300
$REDIS_CLI EXPIRE "cache:page:about" 600

# API data
$REDIS_CLI SET "api/v1/products" '[{"id": 1, "name": "Widget"}, {"id": 2, "name": "Gadget"}]'
$REDIS_CLI SET "api/v1/categories" '["electronics", "clothing", "books"]'

# Lists
$REDIS_CLI RPUSH "queue:tasks" "task1" "task2" "task3"

# Sets
$REDIS_CLI SADD "tags:post:1" "rust" "tui" "redis"

# Hashes
$REDIS_CLI HSET "session:abc123" "user_id" "1" "created" "2024-01-01" "ip" "192.168.1.1"

# Sorted sets
$REDIS_CLI ZADD "leaderboard:daily" 100 "alice" 85 "bob" 72 "charlie"

# Binary-ish data (base64 encoded)
$REDIS_CLI SET "binary:sample" "$(echo -n 'SGVsbG8gV29ybGQh' | base64 -d)"

echo "Done! Seeded $($REDIS_CLI DBSIZE | awk '{print $2}') keys"
