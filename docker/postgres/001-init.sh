#!/bin/bash
set -e
set -u

# These environment variables come from docker-compose
DB_NAME=${APP_DATABASE__DATABASE_NAME}
DB_USER=${APP_DATABASE__USERNAME}
DB_PASSWORD=${APP_DATABASE__PASSWORD}

echo "Creating database user: ${DB_USER}"
echo "Creating database: ${DB_NAME}"

# Create user and database
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<EOF
-- Create application user with limited privileges
CREATE USER ${DB_USER} WITH PASSWORD '${DB_PASSWORD}';
CREATE DATABASE "${DB_NAME}";
GRANT CONNECT ON DATABASE "${DB_NAME}" TO ${DB_USER};
EOF

# Set up schema permissions - app user gets data access permissions
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "${DB_NAME}" <<EOF
-- Create extensions (as postgres user)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Application user gets data manipulation privileges only
GRANT USAGE ON SCHEMA public TO ${DB_USER};
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO ${DB_USER};
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO ${DB_USER};

-- Set default privileges for future objects
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ${DB_USER};
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT USAGE, SELECT ON SEQUENCES TO ${DB_USER};

-- Ensure _sqlx_migrations table is readable
-- SQLx needs to be able to read the migration status
CREATE TABLE IF NOT EXISTS _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    success BOOLEAN NOT NULL,
    checksum BYTEA NOT NULL,
    execution_time BIGINT NOT NULL
);
GRANT SELECT ON _sqlx_migrations TO ${DB_USER};
EOF

echo "Database setup complete"
echo "App user: ${DB_USER} - For application connections"
echo "Admin user (postgres): Use for migrations and administration"
