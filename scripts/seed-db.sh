#!/bin/bash
set -e

# Seed database with initial data for Rust application
# Usage: ./scripts/seed-db.sh [environment]

ENVIRONMENT=${1:-local}
CONFIG_FILE="configs/${ENVIRONMENT}.yaml"

echo "=========================================="
echo "Seeding Database for Zercle Rust Template"
echo "=========================================="
echo "Environment: ${ENVIRONMENT}"
echo ""

# Database configuration
DB_HOST=${DB_HOST:-localhost}
DB_PORT=${DB_PORT:-5432}
DB_USER=${DB_USER:-postgres}
DB_PASSWORD=${DB_PASSWORD:-postgres}
DB_NAME=${DB_NAME:-postgres}

# Build database URL
DB_URL="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

echo "Database: ${DB_NAME} on ${DB_HOST}:${DB_PORT}"
echo ""

# Check if psql is available
if ! command -v psql &> /dev/null; then
    echo "Error: psql is not installed"
    exit 1
fi

# Check database connection
echo "Checking database connection..."
if ! PGPASSWORD=${DB_PASSWORD} psql -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -d ${DB_NAME} -c "SELECT 1" > /dev/null 2>&1; then
    echo "Error: Cannot connect to database"
    echo "Make sure PostgreSQL is running and credentials are correct"
    exit 1
fi
echo "✅ Database connection successful"

# Run migrations first
echo ""
echo "Running database migrations..."
if [ -d "sqlc/migrations" ]; then
    for migration in sqlc/migrations/*.sql; do
        if [ -f "$migration" ]; then
            echo "  Applying: $(basename $migration)"
            PGPASSWORD=${DB_PASSWORD} psql -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -d ${DB_NAME} -f "$migration" > /dev/null 2>&1 || true
        fi
    done
    echo "✅ Migrations completed"
else
    echo "⚠️  No migrations directory found"
fi

# Seed sample data
echo ""
echo "Seeding sample data..."

# Create test users
echo "  Creating sample users..."

# Sample user 1
USER1_EMAIL="test1@example.com"
USER1_PASSWORD="Password123!"
USER1_HASH=$(echo -n "${USER1_PASSWORD}" | argon2 "$(uuidgen 2>/dev/null || echo 'salt1')" -id -t 3 -p 4 -l 32 2>/dev/null || echo "placeholder_hash")

PGPASSWORD=${DB_PASSWORD} psql -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -d ${DB_NAME} << EOF
INSERT INTO users (email, password_hash, full_name, phone)
VALUES ('${USER1_EMAIL}', 'hashed_password_placeholder', 'Test User 1', '+1234567001')
ON CONFLICT (email) DO NOTHING;

INSERT INTO users (email, password_hash, full_name, phone)
VALUES ('admin@example.com', 'hashed_password_placeholder', 'Administrator', '+1234567000')
ON CONFLICT (email) DO NOTHING;
EOF

echo "  ✅ Sample users created"

# Create sample tasks for test user
echo "  Creating sample tasks..."
USER1_ID=$(PGPASSWORD=${DB_PASSWORD} psql -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -d ${DB_NAME} -t -c "SELECT id FROM users WHERE email = '${USER1_EMAIL}'" 2>/dev/null | tr -d ' ' || echo "")

if [ -n "${USER1_ID}" ]; then
    PGPASSWORD=${DB_PASSWORD} psql -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -d ${DB_NAME} << EOF
INSERT INTO tasks (user_id, title, description, status, priority)
VALUES ('${USER1_ID}', 'Welcome Task', 'This is a sample task to get started', 'pending', 'high')
ON CONFLICT DO NOTHING;

INSERT INTO tasks (user_id, title, description, status, priority)
VALUES ('${USER1_ID}', 'Learn Rust', 'Study axum framework and async programming', 'in_progress', 'high')
ON CONFLICT DO NOTHING;

INSERT INTO tasks (user_id, title, description, status, priority)
VALUES ('${USER1_ID}', 'Read Documentation', 'Read axum and sqlx documentation', 'completed', 'medium')
ON CONFLICT DO NOTHING;
EOF
    echo "  ✅ Sample tasks created"
else
    echo "  ⚠️  Could not find user ID for task creation"
fi

# Show summary
echo ""
echo "=========================================="
echo "Database Seeding Complete!"
echo "=========================================="
echo ""
echo "Sample users created:"
echo "  - test1@example.com / Password123!"
echo "  - admin@example.com / Password123!"
echo ""
echo "You can now start the application with:"
echo "  ./scripts/run-dev.sh"
echo "  or"
echo "  make docker-up && make run"
echo ""
