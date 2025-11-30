BEGIN;
CREATE TABLE IF NOT EXISTS songs (
    path        TEXT NOT NULL,
    mtime       TEXT NOT NULL,
    generation  INTEGER DEFAULT 0,
    title       TEXT,
    artist      TEXT,
    album       TEXT
    -- TODO: update as we add more tags
    -- TODO: playcount/skipcount/added to library timestamp
);
CREATE TABLE IF NOT EXISTS state (
    generation  INTEGER DEFAULT 0,

    current     INTEGER DEFAULT 0,
    head        INTEGER DEFAULT 0,
    tail        INTEGER DEFAULT 0,

    repeat      BOOLEAN DEFAULT 0,
    random      BOOLEAN DEFAULT 0,
    single      BOOLEAN DEFAULT 0,
    consume     BOOLEAN DEFAULT 0
);
INSERT OR IGNORE INTO state (rowid) VALUES (0);

CREATE TABLE IF NOT EXISTS queue (
    -- can't use song as primary key, need to support duplicates
    song_id INTEGER PRIMARY KEY AUTOINCREMENT,
    song    INTEGER, -- rowid in songs
    next    INTEGER,
    prev    INTEGER
);
COMMIT;
