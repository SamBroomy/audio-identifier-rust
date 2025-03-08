

migrate:
    sqlx database drop -y
    sqlx database create
    sqlx migrate run
