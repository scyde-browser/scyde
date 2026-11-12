//! Internal pages (`scyde://…`) and their bridge (CLAUDE.md rule 6).
//!
//! WebView2 exposes the `scyde` custom protocol as `http://scyde.localhost/`. Pages are the
//! built `web/apps/*` bundled as resources. `bridge` answers only webviews whose current URL is
//! on that origin, and only methods in `METHODS` (the same list as `@scyde/bridge`).

use std::borrow::Cow;

use serde_json::{Value, json};
use tauri::http::{Request, Response};
use tauri::{AppHandle, Manager, UriSchemeContext, Webview};

use crate::{AppState, NEWTAB, emit_state, go, lock, save_settings};

pub const PAGES: &[&str] = &["newtab", "settings", "shields", "library", "welcome"];
const ORIGIN: &str = "http://scyde.localhost/";

/// The path of an internal page URL (`/settings/`), or `None` for anything else.
pub fn page_path(url: &str) -> Option<&str> {
    url.strip_prefix("http://scyde.localhost").or_else(|| url.strip_prefix("https://scyde.localhost"))
}

/// `scyde://settings` → `http://scyde.localhost/settings/`. Unknown pages go to New Tab.
pub fn to_protocol_url(url: &str) -> Option<String> {
    let rest = url.strip_prefix("scyde://")?;
    let page = rest.trim_matches('/').split(['/', '?', '#']).next().unwrap_or("");
    let page = if PAGES.contains(&page) { page } else { "newtab" };
    Some(format!("{ORIGIN}{page}/"))
}

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; \
                   font-src 'self'; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; \
                   form-action 'none'; frame-ancestors 'none'";

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "woff2" => "font/woff2",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        _ => "application/octet-stream",
    }
}

/// Protocol handler: `/<page>/<file>` from `resources/pages/<page>/`.
pub fn serve(ctx: UriSchemeContext<'_, tauri::Wry>, request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path().to_owned();
    let mut parts = path.trim_start_matches('/').splitn(2, '/');
    let page = parts.next().unwrap_or("");
    let file = parts.next().filter(|f| !f.is_empty()).unwrap_or("index.html");
    let not_found = || Response::builder().status(404).body(Cow::Borrowed(&b"not found"[..])).expect("response");
    if !PAGES.contains(&page) || file.contains("..") || file.contains('\\') {
        return not_found();
    }
    let Ok(base) = ctx.app_handle().path().resource_dir() else { return not_found() };
    match std::fs::read(base.join("pages").join(page).join(file)) {
        Ok(bytes) => Response::builder()
            .status(200)
            .header("Content-Type", mime(file))
            .header("Content-Security-Policy", CSP)
            .header("X-Content-Type-Options", "nosniff")
            .header("Referrer-Policy", "no-referrer")
            .header("Cache-Control", "no-cache")
            .body(Cow::Owned(bytes))
            .expect("response"),
        Err(_) => not_found(),
    }
}

pub const METHODS: &[&str] = &[
    "newtab.state",
    "navigate",
    "settings.schema",
    "settings.get",
    "settings.set",
    "settings.reset",
    "history.query",
    "history.delete",
    "history.clear",
    "bookmarks.children",
    "bookmarks.remove",
    "shield.summary",
    "shield.setSite",
];

fn ok(id: &Value, result: Value) -> Value {
    json!({ "id": id, "ok": true, "result": result })
}

fn err(id: &Value, e: impl std::fmt::Display) -> Value {
    json!({ "id": id, "ok": false, "error": e.to_string() })
}

/// The bridge for internal pages. Refuses any webview that is not a tab currently showing an
/// internal page, and any method outside the allowlist.
#[tauri::command]
pub fn bridge(webview: Webview, app: AppHandle, message: Value) -> Value {
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let trusted = webview.label().starts_with("tab-")
        && webview.url().is_ok_and(|u| u.scheme() == "http" && u.host_str() == Some("scyde.localhost"));
    if !trusted {
        return err(&id, "untrusted origin");
    }
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    if !METHODS.contains(&method) {
        return err(&id, format!("method not allowed: {method}"));
    }
    let p = message.get("params").cloned().unwrap_or(Value::Null);
    match call(&app, method, &p) {
        Ok(v) => ok(&id, v),
        Err(e) => err(&id, e),
    }
}

fn s(p: &Value, k: &str) -> String {
    p.get(k).and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn call(app: &AppHandle, method: &str, p: &Value) -> Result<Value, String> {
    let state = app.state::<AppState>();
    let places_err = |e: places::PlacesError| e.to_string();
    match method {
        "newtab.state" => {
            let places = lock(&state.places);
            let recent = places
                .as_ref()
                .map(|pl| pl.history(None, 6, 0).unwrap_or_default())
                .unwrap_or_default()
                .into_iter()
                .map(|h| json!({ "url": h.url, "title": h.title, "lastVisit": h.last_visit }))
                .collect::<Vec<_>>();
            let top = places
                .as_ref()
                .map(|pl| pl.top_sites(8).unwrap_or_default())
                .unwrap_or_default()
                .into_iter()
                .map(|t| json!({ "url": t.url, "title": t.title, "pinned": t.pinned }))
                .collect::<Vec<_>>();
            let popups: u32 = lock(&state.tabs).list.iter().map(|t| t.popups_blocked).sum();
            let stories = lock(&state.settings).get("newtab.top_stories").and_then(Value::as_bool).unwrap_or(false);
            // DNR does not report per-request matches in release builds (ADR-0003); only popups are counted.
            Ok(json!({ "shield": { "blocked": popups, "bytesSaved": 0, "msSaved": 0 }, "recent": recent, "topSites": top, "storiesEnabled": stories }))
        }
        "navigate" => {
            let input = s(p, "input");
            let new_tab = p.get("newTab").and_then(Value::as_bool).unwrap_or(false);
            let app = app.clone();
            std::thread::spawn(move || go(&app, &input, new_tab));
            Ok(json!({ "ok": true }))
        }
        "settings.schema" => Ok(json!({ "schema": serde_json::from_str::<Value>(settings::SCHEMA_JSON).unwrap_or(Value::Null) })),
        "settings.get" => {
            let json = lock(&state.settings).to_json();
            Ok(json!({ "settings": serde_json::from_str::<Value>(&json).unwrap_or(Value::Null), "issues": [] }))
        }
        "settings.set" => {
            let path = s(p, "path");
            let value = p.get("value").cloned().unwrap_or(Value::Null);
            let done = path == "onboarding.completed" && value == Value::Bool(true);
            let r = lock(&state.settings).set(&path, value);
            match r {
                Ok(()) => {
                    save_settings(app);
                    emit_state(app);
                    // The welcome page is finished: its tab becomes a New Tab.
                    if done {
                        let welcome: Vec<_> = lock(&state.tabs)
                            .list
                            .iter()
                            .filter(|t| page_path(&t.url).is_some_and(|p| p.starts_with("/welcome")))
                            .map(|t| t.webview.clone())
                            .collect();
                        if let Ok(url) = NEWTAB.parse::<tauri::Url>() {
                            for wv in welcome {
                                let _ = wv.navigate(url.clone());
                            }
                        }
                    }
                    Ok(json!({ "ok": true }))
                }
                Err(e) => Ok(json!({ "ok": false, "error": e.to_string() })),
            }
        }
        "settings.reset" => {
            lock(&state.settings).reset(&s(p, "path")).map_err(|e| e.to_string())?;
            save_settings(app);
            emit_state(app);
            Ok(json!({ "ok": true }))
        }
        "history.query" => {
            let places = lock(&state.places);
            let Some(pl) = places.as_ref() else { return Ok(json!({ "entries": [] })) };
            let q = p.get("query").and_then(Value::as_str);
            let limit = u32::try_from(p.get("limit").and_then(Value::as_u64).unwrap_or(50)).unwrap_or(50).min(500);
            let offset = u32::try_from(p.get("offset").and_then(Value::as_u64).unwrap_or(0)).unwrap_or(0);
            let entries: Vec<Value> = pl
                .history(q, limit, offset)
                .map_err(places_err)?
                .into_iter()
                .map(|h| json!({ "url": h.url, "title": h.title, "lastVisit": h.last_visit, "visitCount": h.visit_count }))
                .collect();
            Ok(json!({ "entries": entries }))
        }
        "history.delete" => {
            if let Some(pl) = lock(&state.places).as_mut() {
                pl.delete_url(&s(p, "url")).map_err(places_err)?;
            }
            Ok(json!({ "ok": true }))
        }
        "history.clear" => {
            if let Some(pl) = lock(&state.places).as_mut() {
                pl.clear_history().map_err(places_err)?;
            }
            Ok(json!({ "ok": true }))
        }
        "bookmarks.children" => {
            let places = lock(&state.places);
            let Some(pl) = places.as_ref() else { return Ok(json!({ "items": [] })) };
            let parent = p.get("parent").and_then(Value::as_i64).unwrap_or(places::BOOKMARKS_BAR);
            let items: Vec<Value> = pl
                .children(parent)
                .map_err(places_err)?
                .into_iter()
                .map(|b| json!({ "id": b.id, "parent": b.parent, "isFolder": b.is_folder, "url": b.url, "title": b.title }))
                .collect();
            Ok(json!({ "items": items }))
        }
        "bookmarks.remove" => {
            if let Some(pl) = lock(&state.places).as_mut() {
                pl.remove_bookmark(p.get("id").and_then(Value::as_i64).unwrap_or(0)).map_err(places_err)?;
            }
            Ok(json!({ "ok": true }))
        }
        "shield.summary" => {
            let popups: u32 = lock(&state.tabs).list.iter().map(|t| t.popups_blocked).sum();
            let zero = json!({ "blocked": popups, "bytesSaved": 0, "msSaved": 0 });
            Ok(json!({ "today": zero, "total": zero, "filtersVersion": state.filters_version, "sites": [] }))
        }
        "shield.setSite" => Ok(json!({ "ok": false, "error": "Per-site Shield switches arrive in a later update on Windows." })),
        _ => Err(format!("method not allowed: {method}")),
    }
}
