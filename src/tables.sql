BEGIN;
CREATE TABLE IF NOT EXISTS songs (
    path        TEXT NOT NULL,
    mtime       TEXT NOT NULL,
    generation  INTEGER DEFAULT 0,

    -- incremented when you get half way through a song
    play_count  INTEGER DEFAULT 0,
    -- incremented when you skip a song in the first half
    skip_count  INTEGER DEFAULT 0,
    date_added  TEXT DEFAULT CURRENT_TIMESTAMP,


    duration            FLOAT,
    title               TEXT,
    artist              TEXT,
    artist_sort         TEXT,
    album               TEXT,
    album_sort          TEXT,
    album_artist        TEXT,
    album_artist_sort   TEXT,
    title_sort          TEXT,
    track               INTEGER,
    name                TEXT,
    genre               TEXT,
    mood                TEXT,
    date                TEXT,
    original_date       TEXT,
    composer            TEXT,
    composer_sort       TEXT,
    performer           TEXT,
    conductor           TEXT,
    work                TEXT,
    ensemble            TEXT,
    movement            TEXT,
    movement_number     TEXT,
    show_movement       BOOLEAN,
    location            TEXT,
    grouping            TEXT,
    comment             TEXT,
    disc                INTEGER,
    label               TEXT,

    musicbrainz_artist_id           TEXT,
    musicbrainz_album_id            TEXT,
    musicbrainz_album_artist_id     TEXT,
    musicbrainz_track_id            TEXT,
    musicbrainz_releasegroup_id     TEXT,
    musicbrainz_release_track_id    TEXT,
    musicbrainz_work_id             TEXT
);

-- this makes scanning significantly faster at the cost of a bit of extra space (~15% larger database)
-- CREATE UNIQUE INDEX IF NOT EXISTS idx_songs ON songs (path);

CREATE TABLE IF NOT EXISTS state (
    -- used to remove deleted songs
    generation  INTEGER DEFAULT 0,

    -- position in queue
    current     INTEGER DEFAULT 0, -- TODO: also store songid of current

    repeat      BOOLEAN DEFAULT 0,
    random      BOOLEAN DEFAULT 0,
    single      BOOLEAN DEFAULT 0,
    consume     BOOLEAN DEFAULT 0
);
INSERT OR IGNORE INTO state (rowid) VALUES (0);

CREATE TABLE IF NOT EXISTS queue (
    -- can't use song as primary key, need to support duplicates
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    song        INTEGER, -- rowid in songs table
    position    INTEGER,
    prio        INTEGER DEFAULT 0,
    range_start FLOAT,
    range_end   FLOAT
);
COMMIT;
