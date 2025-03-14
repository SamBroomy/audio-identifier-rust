# Database configuration

db_port := "5432"
superuser := "postgres"
superuser_pwd := "password"
app_user := "app"
app_user_pwd := "secret"
app_db_name := "audioIdentifier"
export DATABASE_URL := "postgres://" + app_user + ":" + app_user_pwd + "@localhost:" + db_port / app_db_name

bootstrap: install-pre-commit init_db

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

# Check if sqlx is installed
check-sqlx:
    #!/usr/bin/env sh
    if ! [ -x "$(command -v sqlx)" ]; then
        echo >&2 "Error: sqlx is not installed."
        echo >&2 "Use:"
        echo >&2 "    cargo install --version='~0.8' sqlx-cli --no-default-features --features rustls,postgres"
        echo >&2 "to install it."
        exit 1
    fi

# Check if a postgres container is already running
check-postgres-running:
    #!/usr/bin/env sh
    RUNNING=$(docker ps --filter 'name=postgres' --format '{{{{.ID}}')
    if [[ -n $RUNNING ]]; then
        echo >&2 "There is a postgres container already running, kill it with:"
        echo >&2 "    docker kill $RUNNING"
        exit 1
    fi

start-postgres: check-postgres-running
    #!/usr/bin/env sh
    CONTAINER_NAME="postgres_$(date '+%s')"
    docker run \
        --env POSTGRES_USER={{ superuser }} \
        --env POSTGRES_PASSWORD={{ superuser_pwd }} \
        --health-cmd="pg_isready -U {{ superuser }} || exit 1" \
        --health-interval=1s \
        --health-timeout=5s \
        --health-retries=5 \
        --publish {{ db_port }}:5432 \
        --detach \
        --name "${CONTAINER_NAME}" \
        postgres -N 1000

    # Wait for container to be healthy
    until [ "$(docker inspect -f '{{{{.State.Health.Status}}' ${CONTAINER_NAME})" == "healthy" ]; do
        echo >&2 "Postgres is still unavailable - sleeping"
        sleep 1
    done

    echo "${CONTAINER_NAME}" > .postgres-container-id

# Create application user and grant privileges
setup-db-user:
    #!/usr/bin/env sh
    CONTAINER=$(cat .postgres-container-id)
    docker exec -it "${CONTAINER}" psql -U {{ superuser }} -c "CREATE USER {{ app_user }} WITH PASSWORD '{{ app_user_pwd }}';"
    docker exec -it "${CONTAINER}" psql -U {{ superuser }} -c "ALTER USER {{ app_user }} CREATEDB;"

# Create database and run migrations
setup-database: check-sqlx
    sqlx database create
    sqlx migrate run
    cargo sqlx prepare

# Initialize the database (main entry point)
init_db: check-sqlx
    #!/usr/bin/env sh

    set -eo pipefail

    # Export the DATABASE_URL to a .env file
    echo "DATABASE_URL={{ DATABASE_URL }}" > .env


    if [[ -z "${SKIP_DOCKER}" ]]; then
        just start-postgres
        just setup-db-user
    fi
    just setup-database
    echo >&2 "Postgres has been migrated, ready to go!"

# Migrate the database without starting Docker
migrate:
    SKIP_DOCKER=true just init_db

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
    curl http://127.0.0.1:8000/health_check -v

songs:
    curl --request POST \
    --data 'title=My%20Song&artist=Me' \
    127.0.0.1:8000/song --verbose
