# Database configuration

set dotenv-required := true
set dotenv-load := true

bootstrap: install-pre-commit

install-pre-commit:
    #!/usr/bin/env sh
    if ! command -v prefligit &> /dev/null; then
        echo "Installing prefligit..."
        cargo install --locked --git https://github.com/j178/prefligit
    else
        echo "prefligit is already installed"
    fi
    prefligit install
    prefligit run --all-files

start-postgres:
    #!/usr/bin/env sh
    docker compose up postgres -d

    CONTAINER_NAME=$(docker ps --filter 'name=postgres' --format '{{{{.ID}}')
    # Wait for container to be healthy
    echo >&2 "Postgres is still unavailable - sleeping"
    until [ "$(docker inspect -f '{{{{.State.Health.Status}}' ${CONTAINER_NAME})" == "healthy" ]; do
        sleep .1
    done

# Check if sqlx is installed
check-sqlx:
    #!/usr/bin/env sh
    if ! [ -x "$(command -v sqlx)" ]; then
        echo >&2 "Error: sqlx is not installed. Installing..."
        cargo install sqlx-cli --features postgres,rustls
    fi

# Create database and run migrations
setup-database: check-sqlx
    sqlx database create
    sqlx migrate run
    cargo sqlx prepare --all

db_run: start-postgres setup-database

dev: db_run
    docker compose up -d --build

down:
    docker compose down

reload-server:
    docker compose build app
    docker compose up -d app

down-v:
    docker compose down -v

restart-v: down-v dev

# Stop the running postgres container
stop_db:
    #!/usr/bin/env sh
    if [ -f .postgres-container-id ]; then
        docker stop $(cat .postgres-container-id)
        rm .postgres-container-id
    else
        docker stop $(docker ps --filter 'name=postgres' --format '{{{{.ID}}')
    fi

migrate_old:
    sqlx database drop -y
    sqlx database create
    sqlx migrate run

quick_dev:
    bacon run -- -q --example quick_dev

bacon:
    bacon run-long

health_check:
    @curl http://${APP_APPLICATION__HOST}:${APP_APPLICATION__PORT}/health --verbose

songs:
    @curl --request POST \
    --data 'title=My%20Song&artist=Me' \
    http://${APP_APPLICATION__HOST}:${APP_APPLICATION__PORT}/song --verbose

subscribe:
    @curl --request POST \
    --data 'name=Hello%20World&email=hello%40world.com' \
    http://${APP_APPLICATION__HOST}:${APP_APPLICATION__PORT}/subscriptions --verbose

subscribe-confirm token:
    @curl --request GET \
    "http://${APP_APPLICATION__HOST}:${APP_APPLICATION__PORT}/subscriptions/confirm?subscription_token={{ token }}" --verbose
