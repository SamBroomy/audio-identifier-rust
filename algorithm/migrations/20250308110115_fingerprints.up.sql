-- Add up migration script here
CREATE TABLE IF NOT EXISTS fingerprints (
    id INTEGER PRIMARY KEY,
    song_id INTEGER NOT NULL,
    hash INTEGER NOT NULL,
    time_offset REAL NOT NULL,
    confidence INTEGER NOT NULL,
    anchor_frequency INTEGER NOT NULL,
    target_frequency INTEGER NOT NULL,
    delta_time REAL NOT NULL,
    FOREIGN KEY (song_id) REFERENCES songs(id)
);
CREATE INDEX IF NOT EXISTS idx_fingerprints_hash ON fingerprints(hash);