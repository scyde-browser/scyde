//! Behaviour and scale tests for `places` (PLAN §11 Phase 1.6, §14.3 "100k history entries").

#![allow(clippy::many_single_char_names)] // short names keep the tree tests readable

use std::time::Instant;

use omnibox::frecency::{MatchQuality, VisitKind};
use places::{BOOKMARKS_BAR, OTHER_BOOKMARKS, Places, PlacesError, schema};

const NOW: i64 = 1_800_000_000;
const DAY: i64 = 86_400;

fn db() -> Places {
    Places::open_in_memory().unwrap()
}

#[test]
fn migrations_create_and_refuse_newer() {
    let dir = std::env::temp_dir().join(format!("scyde-places-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("places.sqlite");
    let _ = std::fs::remove_file(&path);
    {
        let p = Places::open(&path).unwrap();
        assert_eq!(p.schema_version().unwrap(), schema::VERSION);
    }
    // Reopen: idempotent
    assert_eq!(
        Places::open(&path).unwrap().schema_version().unwrap(),
        schema::VERSION
    );
    // A database from a newer build is refused, not downgraded
    {
        let c = rusqlite::Connection::open(&path).unwrap();
        c.pragma_update(None, "user_version", schema::VERSION + 1)
            .unwrap();
    }
    assert!(matches!(
        Places::open(&path),
        Err(PlacesError::NewerSchema { .. })
    ));
    // A v1 database upgrades to the current version
    let v1 = dir.join("v1.sqlite");
    let _ = std::fs::remove_file(&v1);
    {
        let mut p = Places::open(&v1).unwrap();
        p.record_visit(
            "https://before.hr/",
            Some("Before upgrade"),
            VisitKind::Link,
            NOW,
        )
        .unwrap();
        let c = p.connection();
        c.execute_batch(
            "DROP TRIGGER places_fts_ai; DROP TRIGGER places_fts_ad; DROP TRIGGER places_fts_au;
             DROP TABLE places_fts; DROP TABLE pinned_sites; DROP TABLE hidden_top_sites;
             PRAGMA user_version = 1;",
        )
        .unwrap();
    }
    let p = Places::open(&v1).unwrap();
    assert_eq!(p.schema_version().unwrap(), schema::VERSION);
    p.pin_site(0, "https://a.hr/", "A").unwrap();
    // v3 rebuilt the search index from existing rows
    assert_eq!(p.history(Some("before upgrade"), 10, 0).unwrap().len(), 1);
}

#[test]
fn visits_titles_and_history() {
    let mut p = db();
    assert!(
        p.record_visit(
            "https://index.hr/",
            Some("Index"),
            VisitKind::Typed,
            NOW - 2 * DAY
        )
        .unwrap()
    );
    assert!(
        p.record_visit("https://index.hr/", None, VisitKind::Link, NOW - DAY)
            .unwrap()
    );
    assert!(
        p.record_visit("https://github.com/", Some("GitHub"), VisitKind::Link, NOW)
            .unwrap()
    );
    assert!(
        !p.record_visit("scyde://newtab", None, VisitKind::Typed, NOW)
            .unwrap(),
        "internal pages are not history"
    );
    assert!(
        !p.record_visit("about:blank", None, VisitKind::Link, NOW)
            .unwrap()
    );
    p.set_title("https://github.com/", "GitHub · SCYDE")
        .unwrap();

    let h = p.history(None, 10, 0).unwrap();
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].url, "https://github.com/");
    assert_eq!(h[0].title, "GitHub · SCYDE");
    assert_eq!(h[1].visit_count, 2);
    assert_eq!(h[1].title, "Index", "an empty title does not overwrite");
    assert_eq!(p.history(Some("index"), 10, 0).unwrap().len(), 1);
    assert_eq!(
        p.history(Some("100%"), 10, 0).unwrap().len(),
        0,
        "LIKE wildcards are escaped"
    );
    assert_eq!(p.history(None, 1, 1).unwrap()[0].url, "https://index.hr/");
}

#[test]
fn deleting_history_keeps_bookmarks() {
    let mut p = db();
    p.record_visit("https://a.hr/", Some("A"), VisitKind::Link, NOW - 3 * DAY)
        .unwrap();
    p.record_visit("https://a.hr/", Some("A"), VisitKind::Link, NOW - 600)
        .unwrap();
    p.record_visit("https://b.hr/", Some("B"), VisitKind::Link, NOW - 600)
        .unwrap();
    p.add_bookmark(BOOKMARKS_BAR, "https://b.hr/", "B", NOW)
        .unwrap();

    // "Clear the last hour": a.hr keeps its older visit, b.hr has none left
    p.delete_range(NOW - 3600, NOW + 1, NOW).unwrap();
    let h = p.history(None, 10, 0).unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(
        (h[0].url.as_str(), h[0].visit_count, h[0].last_visit),
        ("https://a.hr/", 1, NOW - 3 * DAY)
    );

    p.delete_url("https://a.hr/").unwrap();
    p.clear_history().unwrap();
    assert!(p.history(None, 10, 0).unwrap().is_empty());
    assert!(
        p.is_bookmarked("https://b.hr/").unwrap(),
        "bookmark survives clearing history"
    );
    assert_eq!(
        p.suggest("b.hr", 5).unwrap()[0].url,
        "https://b.hr/",
        "bookmarked page still suggested"
    );
}

#[test]
fn bookmarks_tree() {
    let mut p = db();
    let f = p.add_folder(BOOKMARKS_BAR, "News", NOW).unwrap();
    let a = p.add_bookmark(f, "https://a.hr/", "A", NOW).unwrap();
    let b = p.add_bookmark(f, "https://b.hr/", "B", NOW).unwrap();
    let c = p.add_bookmark(f, "https://c.hr/", "C", NOW).unwrap();
    let titles = |p: &Places, parent| {
        p.children(parent)
            .unwrap()
            .into_iter()
            .map(|b| b.title)
            .collect::<Vec<_>>()
    };
    assert_eq!(titles(&p, f), ["A", "B", "C"]);

    p.move_bookmark(c, f, 0).unwrap();
    assert_eq!(titles(&p, f), ["C", "A", "B"]);
    p.move_bookmark(a, OTHER_BOOKMARKS, 99).unwrap();
    assert_eq!(titles(&p, f), ["C", "B"]);
    assert_eq!(
        p.children(f)
            .unwrap()
            .iter()
            .map(|b| b.position)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(titles(&p, OTHER_BOOKMARKS), ["A"]);

    // A folder cannot move into itself or a descendant; built-ins are fixed
    let sub = p.add_folder(f, "Sub", NOW).unwrap();
    assert!(matches!(
        p.move_bookmark(f, sub, 0),
        Err(PlacesError::InvalidBookmark(_))
    ));
    assert!(matches!(
        p.move_bookmark(f, f, 0),
        Err(PlacesError::InvalidBookmark(_))
    ));
    assert!(p.remove_bookmark(BOOKMARKS_BAR).is_err());
    assert!(
        p.add_bookmark(b, "https://x.hr/", "X", NOW).is_err(),
        "a bookmark is not a folder"
    );

    p.rename_bookmark(b, "Bee").unwrap();
    assert_eq!(titles(&p, f), ["C", "Bee", "Sub"]);
    p.remove_bookmark(f).unwrap();
    assert!(
        !p.is_bookmarked("https://b.hr/").unwrap(),
        "removing a folder removes its bookmarks"
    );
    assert!(p.is_bookmarked("https://a.hr/").unwrap());
}

#[test]
fn suggestions_rank_host_prefix_first() {
    let mut p = db();
    for _ in 0..5 {
        p.record_visit(
            "https://www.index.hr/vijesti",
            Some("Index vijesti"),
            VisitKind::Typed,
            NOW - DAY,
        )
        .unwrap();
    }
    for _ in 0..30 {
        p.record_visit(
            "https://example.org/find",
            Some("Find an index"),
            VisitKind::Link,
            NOW,
        )
        .unwrap();
    }
    p.record_visit(
        "https://indeed.com/",
        Some("Jobs"),
        VisitKind::Link,
        NOW - 100 * DAY,
    )
    .unwrap();
    let s = p.suggest("ind", 5).unwrap();
    assert_eq!(s[0].url, "https://www.index.hr/vijesti");
    assert_eq!(s[0].quality, MatchQuality::HostPrefix);
    assert!(s.iter().any(|x| x.url == "https://example.org/find"));
    assert!(p.suggest("   ", 5).unwrap().is_empty());
}

#[test]
fn top_sites_with_pins_and_hidden() {
    let mut p = db();
    for (url, n) in [
        ("https://a.hr/", 9),
        ("https://a.hr/deep/page", 12),
        ("https://b.hr/", 5),
        ("https://c.hr/", 3),
    ] {
        for _ in 0..n {
            p.record_visit(url, Some(url), VisitKind::Link, NOW)
                .unwrap();
        }
    }
    let t = p.top_sites(8).unwrap();
    assert_eq!(
        t.iter().map(|t| t.url.as_str()).collect::<Vec<_>>(),
        ["https://a.hr/deep/page", "https://b.hr/", "https://c.hr/"],
        "one tile per host"
    );

    p.pin_site(0, "https://scyde.dev/", "SCYDE").unwrap();
    p.hide_top_site("https://b.hr/").unwrap();
    let t = p.top_sites(8).unwrap();
    assert_eq!(t[0].url, "https://scyde.dev/");
    assert!(t[0].pinned);
    assert!(!t.iter().any(|t| t.url.contains("b.hr")));
    p.pin_site(1, "https://b.hr/", "B again").unwrap();
    assert!(
        p.top_sites(8)
            .unwrap()
            .iter()
            .any(|t| t.url == "https://b.hr/" && t.pinned),
        "pinning un-hides"
    );
    p.unpin_site(0).unwrap();
    assert!(
        !p.top_sites(8)
            .unwrap()
            .iter()
            .any(|t| t.url.contains("scyde.dev"))
    );
    assert_eq!(p.top_sites(1).unwrap().len(), 1);
}

#[test]
fn frecency_decays() {
    let mut p = db();
    p.record_visit("https://old.hr/", None, VisitKind::Typed, NOW - 200 * DAY)
        .unwrap();
    p.record_visit("https://new.hr/", None, VisitKind::Link, NOW - DAY)
        .unwrap();
    p.recompute_frecency(NOW).unwrap();
    let s = p.suggest(".hr", 2).unwrap();
    assert_eq!(s[0].url, "https://new.hr/");
}

/// 100k pages with visits: inserts, then measures the queries the UI makes on every keystroke.
#[test]
fn scale_100k() {
    let mut p = db();
    let t = Instant::now();
    {
        let c = p.connection();
        let tx = c.transaction().unwrap();
        {
            let mut ins_place = tx
                .prepare("INSERT INTO places (url, host, title, visit_count, last_visit, frecency) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
                .unwrap();
            let mut ins_visit = tx
                .prepare("INSERT INTO visits (place_id, at, kind) VALUES (?1, ?2, 2)")
                .unwrap();
            let mut seed: u64 = 42;
            for i in 0..100_000i64 {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let host = format!("site{}.example", seed % 9_000);
                let url = format!("https://{host}/article/{i}");
                let at = NOW - i64::try_from(seed % (365 * DAY as u64)).unwrap();
                #[allow(clippy::cast_precision_loss)]
                let fr = (seed % 10_000) as f64;
                ins_place
                    .execute(rusqlite::params![
                        url,
                        host,
                        format!("Article {i} on {host}"),
                        1,
                        at,
                        fr
                    ])
                    .unwrap();
                ins_visit
                    .execute(rusqlite::params![tx.last_insert_rowid(), at])
                    .unwrap();
            }
        }
        tx.commit().unwrap();
    }
    let insert_ms = t.elapsed().as_millis();

    let time = |f: &mut dyn FnMut()| {
        let t = Instant::now();
        for _ in 0..10 {
            f();
        }
        t.elapsed().as_secs_f64() * 100.0 // ms per call
    };
    let suggest_ms = time(&mut || {
        let s = p.suggest("site12", 8).unwrap();
        assert!(!s.is_empty());
    });
    let suggest_title_ms = time(&mut || {
        let _ = p.suggest("article 9999", 8).unwrap();
    });
    let history_ms = time(&mut || assert_eq!(p.history(None, 50, 500).unwrap().len(), 50));
    let search_ms = time(&mut || {
        let _ = p.history(Some("article 4242"), 50, 0).unwrap();
    });
    let top_ms = time(&mut || assert_eq!(p.top_sites(12).unwrap().len(), 12));
    let t = Instant::now();
    p.record_visit(
        "https://site1.example/article/1",
        None,
        VisitKind::Typed,
        NOW,
    )
    .unwrap();
    let visit_ms = t.elapsed().as_secs_f64() * 1e3;

    println!(
        "places 100k: insert {insert_ms} ms · suggest(host) {suggest_ms:.2} ms · suggest(title) {suggest_title_ms:.2} ms · \
         history page {history_ms:.2} ms · history search {search_ms:.2} ms · top sites {top_ms:.2} ms · record_visit {visit_ms:.2} ms"
    );
    // Generous ceilings for debug builds on shared runners; release is far faster.
    assert!(suggest_ms < 100.0, "suggest {suggest_ms} ms");
    assert!(history_ms < 50.0, "history page {history_ms} ms");
    assert!(top_ms < 250.0, "top sites {top_ms} ms");
    assert!(visit_ms < 50.0, "record_visit {visit_ms} ms");
}

#[test]
fn unbookmark_url_removes_every_copy() {
    let mut p = db();
    let f = p.add_folder(BOOKMARKS_BAR, "News", NOW).unwrap();
    p.add_bookmark(BOOKMARKS_BAR, "https://a.hr/", "A", NOW)
        .unwrap();
    p.add_bookmark(f, "https://a.hr/", "A again", NOW).unwrap();
    p.add_bookmark(OTHER_BOOKMARKS, "https://b.hr/", "B", NOW)
        .unwrap();
    assert_eq!(p.unbookmark_url("https://a.hr/").unwrap(), 2);
    assert!(!p.is_bookmarked("https://a.hr/").unwrap());
    assert!(p.is_bookmarked("https://b.hr/").unwrap());
    assert_eq!(p.unbookmark_url("https://a.hr/").unwrap(), 0);
    assert_eq!(p.children(f).unwrap().len(), 0);
}
