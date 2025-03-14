-- Add up migration script here
CREATE TABLE IF NOT EXISTS songs (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT,
    duration NUMERIC,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
