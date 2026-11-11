//! Schema and migrations. `PRAGMA user_version` holds the schema version; each migration runs
//! in one transaction, so a crash mid-upgrade leaves the previous version intact.

use rusqlite::{Connection, Transaction};

/// Ordered migrations; index + 1 is the version a migration produces.
const MIGRATIONS: &[fn(&Transaction<'_>) -> rusqlite::Result<()>] = &[v1, v2, v3];

/// Current schema version.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub const VERSION: i32 = MIGRATIONS.len() as i32;

fn v1(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        "
        CREATE TABLE places (
            id          INTEGER PRIMARY KEY,
            url         TEXT NOT NULL UNIQUE,
            host        TEXT NOT NULL,
            title       TEXT NOT NULL DEFAULT '',
            visit_count INTEGER NOT NULL DEFAULT 0,
            typed_count INTEGER NOT NULL DEFAULT 0,
            last_visit  INTEGER,
            frecency    REAL NOT NULL DEFAULT 0
        );
        CREATE INDEX places_frecency ON places(frecency DESC);
        CREATE INDEX places_host ON places(host);
        CREATE INDEX places_last_visit ON places(last_visit DESC);

        CREATE TABLE visits (
            id       INTEGER PRIMARY KEY,
            place_id INTEGER NOT NULL REFERENCES places(id) ON DELETE CASCADE,
            at       INTEGER NOT NULL,
            kind     INTEGER NOT NULL
        );
        CREATE INDEX visits_place_at ON visits(place_id, at DESC);
        CREATE INDEX visits_at ON visits(at);

        CREATE TABLE bookmarks (
            id       INTEGER PRIMARY KEY,
            parent   INTEGER REFERENCES bookmarks(id) ON DELETE CASCADE,
            is_folder INTEGER NOT NULL,
            place_id INTEGER REFERENCES places(id),
            title    TEXT NOT NULL,
            position INTEGER NOT NULL,
            added    INTEGER NOT NULL
        );
        CREATE INDEX bookmarks_parent ON bookmarks(parent, position);
        CREATE INDEX bookmarks_place ON bookmarks(place_id);
        INSERT INTO bookmarks (id, parent, is_folder, title, position, added) VALUES
            (1, NULL, 1, 'root', 0, 0),
            (2, 1, 1, 'Bookmarks bar', 0, 0),
            (3, 1, 1, 'Other bookmarks', 1, 0);
        ",
    )
}

fn v2(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    // Speed dial (New Tab): pinned tiles and tiles the user removed.
    tx.execute_batch(
        "
        CREATE TABLE pinned_sites (
            position INTEGER PRIMARY KEY,
            url      TEXT NOT NULL,
            title    TEXT NOT NULL
        );
        CREATE TABLE hidden_top_sites (host TEXT PRIMARY KEY);
        ",
    )
}

fn v3(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    // Substring search over URL and title for every keystroke: a trigram index instead of a
    // full scan (100k rows: ~70 ms → <5 ms). External content, kept in sync by triggers.
    tx.execute_batch(
        "
        CREATE VIRTUAL TABLE places_fts USING fts5(
            url, title, content='places', content_rowid='id', tokenize='trigram'
        );
        INSERT INTO places_fts(places_fts) VALUES ('rebuild');
        CREATE TRIGGER places_fts_ai AFTER INSERT ON places BEGIN
            INSERT INTO places_fts(rowid, url, title) VALUES (new.id, new.url, new.title);
        END;
        CREATE TRIGGER places_fts_ad AFTER DELETE ON places BEGIN
            INSERT INTO places_fts(places_fts, rowid, url, title) VALUES ('delete', old.id, old.url, old.title);
        END;
        CREATE TRIGGER places_fts_au AFTER UPDATE OF url, title ON places BEGIN
            INSERT INTO places_fts(places_fts, rowid, url, title) VALUES ('delete', old.id, old.url, old.title);
            INSERT INTO places_fts(rowid, url, title) VALUES (new.id, new.url, new.title);
        END;
        ",
    )
}

/// Brings the database to [`VERSION`]. Refuses databases from a newer build.
///
/// # Errors
/// `SQLite` errors, or [`crate::PlacesError::NewerSchema`].
pub fn migrate(conn: &mut Connection) -> Result<(), crate::PlacesError> {
    let current: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > VERSION {
        return Err(crate::PlacesError::NewerSchema {
            found: current,
            supported: VERSION,
        });
    }
    #[allow(clippy::cast_sign_loss)]
    for (i, m) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        m(&tx)?;
        tx.pragma_update(
            None,
            "user_version",
            i64::try_from(i + 1).unwrap_or(i64::MAX),
        )?;
        tx.commit()?;
    }
    Ok(())
}
