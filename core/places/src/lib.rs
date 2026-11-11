//! History, bookmarks and top sites (`SQLite`, PLAN §2.1).
//!
//! One [`Places`] per profile. Private windows never call into this crate (the host decides).
//! Times are seconds since the Unix epoch, passed in by the caller so tests are deterministic.

use std::fmt;
use std::path::Path;

use omnibox::frecency::{self, Candidate, MatchQuality, Visit, VisitKind};
use rusqlite::{Connection, OptionalExtension, params};

pub mod schema;

/// Errors from the places store.
#[derive(Debug)]
pub enum PlacesError {
    Sqlite(rusqlite::Error),
    /// The database was written by a newer SCYDE; refusing to downgrade it.
    NewerSchema {
        found: i32,
        supported: i32,
    },
    /// Bookmark or folder id does not exist, or the operation is not allowed on it.
    InvalidBookmark(i64),
}

impl fmt::Display for PlacesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "sqlite: {e}"),
            Self::NewerSchema { found, supported } => {
                write!(
                    f,
                    "places schema {found} is newer than supported {supported}"
                )
            }
            Self::InvalidBookmark(id) => write!(f, "invalid bookmark or folder {id}"),
        }
    }
}

impl std::error::Error for PlacesError {}

impl From<rusqlite::Error> for PlacesError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

pub type Result<T> = std::result::Result<T, PlacesError>;

/// Built-in folders.
pub const ROOT: i64 = 1;
pub const BOOKMARKS_BAR: i64 = 2;
pub const OTHER_BOOKMARKS: i64 = 3;

/// One history row.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub last_visit: i64,
    pub visit_count: u32,
}

/// A bookmark or folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    pub id: i64,
    pub parent: i64,
    pub is_folder: bool,
    /// `None` for folders.
    pub url: Option<String>,
    pub title: String,
    pub position: i64,
}

/// A speed-dial tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopSite {
    pub url: String,
    pub title: String,
    pub pinned: bool,
}

/// A ranked omnibox suggestion from history and bookmarks.
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub url: String,
    pub title: String,
    pub quality: MatchQuality,
    pub bookmarked: bool,
}

fn kind_code(k: VisitKind) -> i64 {
    match k {
        VisitKind::Typed => 0,
        VisitKind::Bookmark => 1,
        VisitKind::Link => 2,
        VisitKind::Passive => 3,
    }
}

fn kind_from(code: i64) -> VisitKind {
    match code {
        0 => VisitKind::Typed,
        1 => VisitKind::Bookmark,
        2 => VisitKind::Link,
        _ => VisitKind::Passive,
    }
}

/// Lower-case ASCII host of an absolute URL, without credentials, port or `www.`.
fn host_of(url: &str) -> String {
    let after = url.split_once("://").map_or("", |(_, r)| r);
    let authority = &after[..after.find(['/', '?', '#']).unwrap_or(after.len())];
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if host_port.starts_with('[') {
        &host_port[..host_port.find(']').map_or(host_port.len(), |i| i + 1)]
    } else {
        host_port.split(':').next().unwrap_or("")
    };
    let host = host.to_ascii_lowercase();
    host.strip_prefix("www.").map(str::to_owned).unwrap_or(host)
}

/// Only web pages go into history: not `scyde://`, `about:`, `data:`, `file:`…
fn is_recordable(url: &str) -> bool {
    let lower = url.get(..8).unwrap_or(url).to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

/// Escapes `%`, `_` and `\` for `LIKE … ESCAPE '\'`.
fn like_pattern(q: &str) -> String {
    let mut s = String::with_capacity(q.len() + 2);
    s.push('%');
    for c in q.chars() {
        if matches!(c, '%' | '_' | '\\') {
            s.push('\\');
        }
        s.push(c);
    }
    s.push('%');
    s
}

/// An FTS5 phrase query for `q` (trigram tokenizer: substring match, case-insensitive).
/// `None` when `q` is shorter than a trigram; callers fall back to `LIKE`.
fn fts_phrase(q: &str) -> Option<String> {
    (q.chars().count() >= 3).then(|| format!("\"{}\"", q.replace('"', "\"\"")))
}

/// The places database of one profile.
#[derive(Debug)]
pub struct Places {
    conn: Connection,
}

impl Places {
    /// Opens (creating and migrating if needed) the database at `path`.
    ///
    /// # Errors
    /// `SQLite` errors, or [`PlacesError::NewerSchema`].
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    /// An in-memory database (tests, private profiles that need suggestions without history).
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;",
        )?;
        schema::migrate(&mut conn)?;
        Ok(Self { conn })
    }

    /// Schema version of the open database.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn schema_version(&self) -> Result<i32> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    // ---------------------------------------------------------------- history

    /// Records a visit and updates the page's title, counters and frecency.
    /// Non-web URLs (`scyde://`, `about:`, …) are ignored. Returns whether it was recorded.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn record_visit(
        &mut self,
        url: &str,
        title: Option<&str>,
        kind: VisitKind,
        at: i64,
    ) -> Result<bool> {
        if !is_recordable(url) {
            return Ok(false);
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO places (url, host, title) VALUES (?1, ?2, COALESCE(?3, ''))
             ON CONFLICT(url) DO UPDATE SET title = COALESCE(NULLIF(?3, ''), title)",
            params![url, host_of(url), title],
        )?;
        let id: i64 = tx.query_row("SELECT id FROM places WHERE url = ?1", [url], |r| r.get(0))?;
        tx.execute(
            "INSERT INTO visits (place_id, at, kind) VALUES (?1, ?2, ?3)",
            params![id, at, kind_code(kind)],
        )?;
        let counts = i64::from(kind != VisitKind::Passive);
        tx.execute(
            "UPDATE places SET visit_count = visit_count + ?2,
                               typed_count = typed_count + ?3,
                               last_visit = MAX(COALESCE(last_visit, 0), ?4)
             WHERE id = ?1",
            params![id, counts, i64::from(kind == VisitKind::Typed), at],
        )?;
        update_frecency(&tx, id, at)?;
        tx.commit()?;
        Ok(true)
    }

    /// Sets the title of a known page (titles often arrive after the visit).
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn set_title(&self, url: &str, title: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE places SET title = ?2 WHERE url = ?1",
            params![url, title],
        )?;
        Ok(())
    }

    /// History, newest first, optionally filtered by `query` (URL or title), paged.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn history(
        &self,
        query: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<HistoryEntry>> {
        let q = query.map(str::trim).filter(|q| !q.is_empty());
        let map = |r: &rusqlite::Row<'_>| {
            Ok(HistoryEntry {
                url: r.get(0)?,
                title: r.get(1)?,
                last_visit: r.get(2)?,
                visit_count: r.get(3)?,
            })
        };
        if let Some(phrase) = q.and_then(fts_phrase) {
            let mut stmt = self.conn.prepare_cached(
                "SELECT url, title, last_visit, visit_count FROM places
                 WHERE last_visit IS NOT NULL AND id IN (SELECT rowid FROM places_fts WHERE places_fts MATCH ?1)
                 ORDER BY last_visit DESC LIMIT ?2 OFFSET ?3",
            )?;
            return Ok(stmt
                .query_map(params![phrase, limit, offset], map)?
                .collect::<rusqlite::Result<_>>()?);
        }
        let mut stmt = self.conn.prepare_cached(
            "SELECT url, title, last_visit, visit_count FROM places
             WHERE last_visit IS NOT NULL
               AND (?1 IS NULL OR url LIKE ?1 ESCAPE '\\' OR title LIKE ?1 ESCAPE '\\')
             ORDER BY last_visit DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![q.map(like_pattern), limit, offset], |r| {
            Ok(HistoryEntry {
                url: r.get(0)?,
                title: r.get(1)?,
                last_visit: r.get(2)?,
                visit_count: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Removes a page from history (a bookmark to it stays).
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn delete_url(&mut self, url: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM visits WHERE place_id = (SELECT id FROM places WHERE url = ?1)",
            [url],
        )?;
        tx.execute(
            "UPDATE places SET visit_count = 0, typed_count = 0, last_visit = NULL, frecency = 0 WHERE url = ?1",
            [url],
        )?;
        purge_orphans(&tx)?;
        tx.commit()?;
        Ok(())
    }

    /// Removes visits in `[from, to)` ("clear the last hour"), then fixes counters and frecency.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn delete_range(&mut self, from: i64, to: i64, now: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        let ids: Vec<i64> = {
            let mut s =
                tx.prepare("SELECT DISTINCT place_id FROM visits WHERE at >= ?1 AND at < ?2")?;
            s.query_map(params![from, to], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?
        };
        tx.execute(
            "DELETE FROM visits WHERE at >= ?1 AND at < ?2",
            params![from, to],
        )?;
        for id in ids {
            tx.execute(
                "UPDATE places SET
                   visit_count = (SELECT COUNT(*) FROM visits WHERE place_id = ?1 AND kind != 3),
                   typed_count = (SELECT COUNT(*) FROM visits WHERE place_id = ?1 AND kind = 0),
                   last_visit  = (SELECT MAX(at) FROM visits WHERE place_id = ?1)
                 WHERE id = ?1",
                [id],
            )?;
            update_frecency(&tx, id, now)?;
        }
        purge_orphans(&tx)?;
        tx.commit()?;
        Ok(())
    }

    /// Clears all history. Bookmarks, pinned tiles and their pages stay.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn clear_history(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM visits", [])?;
        tx.execute(
            "UPDATE places SET visit_count = 0, typed_count = 0, last_visit = NULL, frecency = 0",
            [],
        )?;
        purge_orphans(&tx)?;
        tx.commit()?;
        Ok(())
    }

    /// Recomputes every frecency for `now` (daily maintenance: scores decay with age).
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn recompute_frecency(&mut self, now: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        let ids: Vec<i64> = {
            let mut s = tx.prepare("SELECT id FROM places")?;
            s.query_map([], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?
        };
        for id in ids {
            update_frecency(&tx, id, now)?;
        }
        tx.commit()?;
        Ok(())
    }

    // ------------------------------------------------------------ suggestions

    /// Omnibox suggestions from history and bookmarks, ranked by match quality and frecency.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn suggest(&self, query: &str, limit: usize) -> Result<Vec<Suggestion>> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let host_q = q.strip_prefix("www.").unwrap_or(&q);
        // Host prefix uses the index; LIKE covers titles and paths. Both capped by frecency.
        let map = |r: &rusqlite::Row<'_>| {
            Ok((
                Candidate {
                    url: r.get(0)?,
                    title: r.get(1)?,
                    frecency: r.get(2)?,
                },
                r.get::<_, bool>(3)?,
            ))
        };
        let rows: Vec<(Candidate, bool)> = if let Some(phrase) = fts_phrase(&q) {
            let mut stmt = self.conn.prepare_cached(
                "SELECT url, title, frecency,
                        EXISTS(SELECT 1 FROM bookmarks b WHERE b.place_id = p.id) AS bm
                 FROM places p
                 WHERE (host >= ?1 AND host < ?1 || x'7f')
                    OR id IN (SELECT rowid FROM places_fts WHERE places_fts MATCH ?2)
                 ORDER BY frecency DESC LIMIT 200",
            )?;
            stmt.query_map(params![host_q, phrase], map)?
                .collect::<rusqlite::Result<_>>()?
        } else {
            let mut stmt = self.conn.prepare_cached(
                "SELECT url, title, frecency,
                        EXISTS(SELECT 1 FROM bookmarks b WHERE b.place_id = p.id) AS bm
                 FROM places p
                 WHERE (host >= ?1 AND host < ?1 || x'7f')
                    OR url LIKE ?2 ESCAPE '\\' OR title LIKE ?2 ESCAPE '\\'
                 ORDER BY frecency DESC LIMIT 200",
            )?;
            stmt.query_map(params![host_q, like_pattern(&q)], map)?
                .collect::<rusqlite::Result<_>>()?
        };
        let bookmarked: std::collections::HashSet<String> = rows
            .iter()
            .filter(|(_, b)| *b)
            .map(|(c, _)| c.url.clone())
            .collect();
        let cands: Vec<Candidate> = rows.into_iter().map(|(c, _)| c).collect();
        Ok(frecency::rank(&q, &cands, limit)
            .into_iter()
            .map(|(c, quality)| Suggestion {
                bookmarked: bookmarked.contains(&c.url),
                url: c.url,
                title: c.title,
                quality,
            })
            .collect())
    }

    // --------------------------------------------------------------- bookmarks

    fn check_folder(&self, id: i64) -> Result<()> {
        let is_folder: Option<bool> = self
            .conn
            .query_row("SELECT is_folder FROM bookmarks WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        if is_folder == Some(true) {
            Ok(())
        } else {
            Err(PlacesError::InvalidBookmark(id))
        }
    }

    fn next_position(&self, parent: i64) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM bookmarks WHERE parent = ?1",
            [parent],
            |r| r.get(0),
        )?)
    }

    /// Adds a bookmark at the end of `parent`. Returns its id.
    ///
    /// # Errors
    /// [`PlacesError::InvalidBookmark`] if `parent` is not a folder; `SQLite` errors.
    pub fn add_bookmark(&mut self, parent: i64, url: &str, title: &str, now: i64) -> Result<i64> {
        self.check_folder(parent)?;
        let pos = self.next_position(parent)?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO places (url, host, title) VALUES (?1, ?2, ?3) ON CONFLICT(url) DO NOTHING",
            params![url, host_of(url), title],
        )?;
        let place: i64 =
            tx.query_row("SELECT id FROM places WHERE url = ?1", [url], |r| r.get(0))?;
        tx.execute(
            "INSERT INTO bookmarks (parent, is_folder, place_id, title, position, added) VALUES (?1, 0, ?2, ?3, ?4, ?5)",
            params![parent, place, title, pos, now],
        )?;
        let id = tx.last_insert_rowid();
        update_frecency(&tx, place, now)?;
        tx.commit()?;
        Ok(id)
    }

    /// Adds a folder at the end of `parent`. Returns its id.
    ///
    /// # Errors
    /// [`PlacesError::InvalidBookmark`] if `parent` is not a folder; `SQLite` errors.
    pub fn add_folder(&self, parent: i64, title: &str, now: i64) -> Result<i64> {
        self.check_folder(parent)?;
        let pos = self.next_position(parent)?;
        self.conn.execute(
            "INSERT INTO bookmarks (parent, is_folder, title, position, added) VALUES (?1, 1, ?2, ?3, ?4)",
            params![parent, title, pos, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Children of a folder, in order.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn children(&self, parent: i64) -> Result<Vec<Bookmark>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT b.id, b.parent, b.is_folder, p.url, b.title, b.position
             FROM bookmarks b LEFT JOIN places p ON p.id = b.place_id
             WHERE b.parent = ?1 ORDER BY b.position",
        )?;
        let rows = stmt.query_map([parent], |r| {
            Ok(Bookmark {
                id: r.get(0)?,
                parent: r.get(1)?,
                is_folder: r.get(2)?,
                url: r.get(3)?,
                title: r.get(4)?,
                position: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Is this URL bookmarked anywhere?
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn is_bookmarked(&self, url: &str) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM bookmarks b JOIN places p ON p.id = b.place_id WHERE p.url = ?1)",
            [url],
            |r| r.get(0),
        )?)
    }

    /// Renames a bookmark or folder.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn rename_bookmark(&self, id: i64, title: &str) -> Result<()> {
        if id <= OTHER_BOOKMARKS {
            return Err(PlacesError::InvalidBookmark(id));
        }
        self.conn.execute(
            "UPDATE bookmarks SET title = ?2 WHERE id = ?1",
            params![id, title],
        )?;
        Ok(())
    }

    /// Moves a bookmark or folder to `index` in `parent` (clamped), renumbering siblings.
    /// A folder cannot move into itself or its descendants; built-in folders cannot move.
    ///
    /// # Errors
    /// [`PlacesError::InvalidBookmark`]; `SQLite` errors.
    pub fn move_bookmark(&mut self, id: i64, parent: i64, index: usize) -> Result<()> {
        if id <= OTHER_BOOKMARKS {
            return Err(PlacesError::InvalidBookmark(id));
        }
        self.check_folder(parent)?;
        let into_self: bool = self.conn.query_row(
            "WITH RECURSIVE up(id) AS (SELECT ?2 UNION SELECT b.parent FROM bookmarks b JOIN up ON b.id = up.id WHERE b.parent IS NOT NULL)
             SELECT EXISTS(SELECT 1 FROM up WHERE id = ?1)",
            params![id, parent],
            |r| r.get(0),
        )?;
        if into_self {
            return Err(PlacesError::InvalidBookmark(id));
        }
        let old_parent: i64 = self
            .conn
            .query_row("SELECT parent FROM bookmarks WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or(PlacesError::InvalidBookmark(id))?;
        let tx = self.conn.transaction()?;
        let mut siblings: Vec<i64> = {
            let mut s = tx.prepare(
                "SELECT id FROM bookmarks WHERE parent = ?1 AND id != ?2 ORDER BY position",
            )?;
            s.query_map(params![parent, id], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?
        };
        siblings.insert(index.min(siblings.len()), id);
        for (pos, sid) in siblings.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            tx.execute(
                "UPDATE bookmarks SET parent = ?2, position = ?3 WHERE id = ?1",
                params![sid, parent, pos as i64],
            )?;
        }
        if old_parent != parent {
            renumber(&tx, old_parent)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Removes a bookmark, or a folder with everything in it.
    ///
    /// # Errors
    /// [`PlacesError::InvalidBookmark`] for built-in folders; `SQLite` errors.
    pub fn remove_bookmark(&mut self, id: i64) -> Result<()> {
        if id <= OTHER_BOOKMARKS {
            return Err(PlacesError::InvalidBookmark(id));
        }
        let tx = self.conn.transaction()?;
        let parent: Option<i64> = tx
            .query_row("SELECT parent FROM bookmarks WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        tx.execute("DELETE FROM bookmarks WHERE id = ?1", [id])?;
        if let Some(p) = parent {
            renumber(&tx, p)?;
        }
        purge_orphans(&tx)?;
        tx.commit()?;
        Ok(())
    }

    // --------------------------------------------------------------- top sites

    /// Speed-dial tiles: pinned tiles at their positions, the rest filled with the best page per
    /// host by frecency, skipping hidden hosts.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn top_sites(&self, limit: usize) -> Result<Vec<TopSite>> {
        let pinned: Vec<(usize, TopSite)> = {
            let mut s = self.conn.prepare_cached(
                "SELECT position, url, title FROM pinned_sites ORDER BY position",
            )?;
            s.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    TopSite {
                        url: r.get(1)?,
                        title: r.get(2)?,
                        pinned: true,
                    },
                ))
            })?
            .filter_map(|r| {
                r.ok()
                    .and_then(|(p, t)| usize::try_from(p).ok().map(|p| (p, t)))
            })
            .collect()
        };
        let pinned_hosts: std::collections::HashSet<String> =
            pinned.iter().map(|(_, t)| host_of(&t.url)).collect();
        let mut s = self.conn.prepare_cached(
            "SELECT url, title, host FROM places p
             WHERE frecency > 0 AND host != ''
               AND host NOT IN (SELECT host FROM hidden_top_sites)
               AND p.id = (SELECT id FROM places q WHERE q.host = p.host
                           ORDER BY q.frecency DESC, length(q.url) LIMIT 1)
             ORDER BY frecency DESC LIMIT ?1",
        )?;
        #[allow(clippy::cast_possible_wrap)]
        let mut fill = s
            .query_map([(limit + pinned.len()) as i64], |r| {
                Ok((
                    TopSite {
                        url: r.get(0)?,
                        title: r.get(1)?,
                        pinned: false,
                    },
                    r.get::<_, String>(2)?,
                ))
            })?
            .filter_map(std::result::Result::ok)
            .filter(|(_, h)| !pinned_hosts.contains(h))
            .map(|(t, _)| t);
        let mut out: Vec<TopSite> = Vec::with_capacity(limit);
        let mut pinned_iter = pinned.into_iter().peekable();
        while out.len() < limit {
            let slot = out.len();
            if let Some((_, t)) = pinned_iter.next_if(|(p, _)| *p <= slot) {
                out.push(t);
            } else if let Some(t) = fill.next() {
                out.push(t);
            } else if let Some((_, t)) = pinned_iter.next() {
                out.push(t);
            } else {
                break;
            }
        }
        Ok(out)
    }

    /// Pins a tile at `position` (replacing what was pinned there).
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn pin_site(&self, position: u32, url: &str, title: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO pinned_sites (position, url, title) VALUES (?1, ?2, ?3)
             ON CONFLICT(position) DO UPDATE SET url = ?2, title = ?3",
            params![position, url, title],
        )?;
        self.conn.execute(
            "DELETE FROM hidden_top_sites WHERE host = ?1",
            [host_of(url)],
        )?;
        Ok(())
    }

    /// Unpins whatever is at `position`.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn unpin_site(&self, position: u32) -> Result<()> {
        self.conn
            .execute("DELETE FROM pinned_sites WHERE position = ?1", [position])?;
        Ok(())
    }

    /// Removes a tile: unpins it and hides its host from the automatic fill.
    ///
    /// # Errors
    /// `SQLite` errors.
    pub fn hide_top_site(&self, url: &str) -> Result<()> {
        let host = host_of(url);
        self.conn
            .execute("DELETE FROM pinned_sites WHERE url = ?1", [url])?;
        self.conn.execute(
            "INSERT OR IGNORE INTO hidden_top_sites (host) VALUES (?1)",
            [host],
        )?;
        Ok(())
    }

    /// Low-level connection, for bulk imports (Phase 7) and tests.
    pub fn connection(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

fn update_frecency(tx: &rusqlite::Transaction<'_>, id: i64, now: i64) -> rusqlite::Result<()> {
    let (total, bookmarked): (u32, bool) = tx.query_row(
        "SELECT visit_count, EXISTS(SELECT 1 FROM bookmarks WHERE place_id = ?1) FROM places WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let visits: Vec<Visit> = {
        #[allow(clippy::cast_possible_wrap)]
        let sample = frecency::SAMPLE as i64;
        let mut s = tx.prepare_cached(
            "SELECT at, kind FROM visits WHERE place_id = ?1 ORDER BY at DESC LIMIT ?2",
        )?;
        s.query_map(params![id, sample], |r| {
            Ok(Visit {
                at: r.get(0)?,
                kind: kind_from(r.get(1)?),
            })
        })?
        .collect::<rusqlite::Result<_>>()?
    };
    let f = frecency::frecency(&visits, total, bookmarked, now);
    tx.execute(
        "UPDATE places SET frecency = ?2 WHERE id = ?1",
        params![id, f],
    )?;
    Ok(())
}

/// Pages with no visits, no bookmark and no pin are deleted.
fn purge_orphans(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute(
        "DELETE FROM places WHERE last_visit IS NULL
           AND id NOT IN (SELECT place_id FROM bookmarks WHERE place_id IS NOT NULL)
           AND url NOT IN (SELECT url FROM pinned_sites)",
        [],
    )?;
    Ok(())
}

fn renumber(tx: &rusqlite::Transaction<'_>, parent: i64) -> rusqlite::Result<()> {
    tx.execute(
        "UPDATE bookmarks SET position = (
            SELECT COUNT(*) FROM bookmarks b2 WHERE b2.parent = bookmarks.parent AND
              (b2.position < bookmarks.position OR (b2.position = bookmarks.position AND b2.id < bookmarks.id)))
         WHERE parent = ?1",
        [parent],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_and_recordable() {
        assert_eq!(
            host_of("https://user:pw@WWW.Example.com:8080/x"),
            "example.com"
        );
        assert_eq!(host_of("http://[::1]:3000/"), "[::1]");
        assert!(is_recordable("HTTPS://x.com"));
        assert!(!is_recordable("scyde://newtab"));
        assert!(!is_recordable("about:blank"));
        assert_eq!(like_pattern("50%_a\\"), "%50\\%\\_a\\\\%");
    }
}
