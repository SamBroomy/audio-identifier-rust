

migrate:
    sqlx database drop -y
    sqlx database create
    sqlx migrate run

quick_dev:
    bacon run -- -q --example quick_dev