//! SCYDE for Windows: Tauri 2 multiwebview on one WebView2 environment (PLAN §9.1, ADR-0007, ADR-0012).
//!
//! Layout: native window frame (Snap Layouts for free) · tab webviews fill the window above the
//! island band · the `chrome` webview (web/apps/desktop-chrome) is the island band at the bottom.
//! Internal pages are served by the `scyde` protocol (`http://scyde.localhost/<page>/` in WebView2)
//! and reach the host only through `bridge`, which checks the origin and a method allowlist.
//! Shield = the bundled MV3 extension (DNR + cosmetics, ADR-0003) on every webview; popups that the
//! user did not initiate are blocked in `NewWindowRequested` (PLAN §5.5).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod internal;
#[cfg(windows)]
mod win;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use omnibox::frecency::VisitKind;
use serde::Serialize;
use serde_json::{Value, json};
use tauri::webview::PageLoadEvent;
use tauri::window::WindowBuilder;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Url,
    Webview, WebviewBuilder, WebviewUrl, WebviewWindowBuilder, Window, WindowEvent,
};

/// Island band height: island 60 + 22 inset below + 22 above (DESIGN §4).
pub const CHROME_H: f64 = 104.0;
/// The island window's width: `size.island-width-expanded` (760) plus room for its shadow.
const ISLAND_W: f64 = 784.0;
/// Where the island window waits while it is hidden.
const OFFSCREEN: f64 = -32000.0;
const START_W: f64 = 1280.0;
const START_H: f64 = 820.0;
pub const NEWTAB: &str = "http://scyde.localhost/newtab/";

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ------------------------------------------------------------------------------------ state

pub struct Tab {
    pub id: u32,
    pub webview: Webview,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub popups_blocked: u32,
    /// The next committed navigation came from the omnibox (frecency: typed).
    pub typed: bool,
}

#[derive(Default)]
pub struct Tabs {
    pub list: Vec<Tab>,
    pub active: Option<u32>,
    pub next_id: u32,
    pub closed: Vec<String>,
}

pub struct AppState {
    pub tabs: Mutex<Tabs>,
    pub places: Mutex<Option<places::Places>>,
    pub settings: Mutex<settings::Settings>,
    pub settings_path: PathBuf,
    pub extensions: Option<PathBuf>,
    pub filters_version: String,
    /// The island is hidden while the page scrolls down (DESIGN v2.1 "Hide").
    pub island_hidden: Mutex<bool>,
    /// The latest scroll sample of a tab: (tab id, scrollY, at the end of the page).
    pub scroll: Mutex<Option<(u32, f64, bool)>>,
}

static SMOKE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Every webview must share the same environment options (ADR-0003, ADR-0007).
fn env_opts(b: WebviewBuilder<tauri::Wry>, state: &AppState) -> WebviewBuilder<tauri::Wry> {
    match &state.extensions {
        Some(dir) => b.browser_extensions_enabled(true).extensions_path(dir),
        None => b,
    }
}

// ------------------------------------------------------------------------------- chrome state

#[derive(Serialize, Clone)]
struct TabView {
    id: u32,
    title: String,
    url: String,
    /// What the address shows at rest (internal pages as `scyde://…`).
    display: Display,
    loading: bool,
    popups_blocked: u32,
}

#[derive(Serialize, Clone, Default)]
struct Display {
    internal: bool,
    scheme: String,
    subdomain: String,
    domain: String,
    rest: String,
    punycode: bool,
}

fn display_for(url: &str) -> Display {
    if let Some(rest) = internal::page_path(url) {
        let page = rest.trim_matches('/').split('/').next().unwrap_or("newtab");
        return Display {
            internal: true,
            scheme: "scyde".into(),
            domain: page.to_owned(),
            ..Display::default()
        };
    }
    omnibox::display_url(url).map_or_else(
        || Display {
            rest: url.to_owned(),
            ..Display::default()
        },
        |d| Display {
            internal: false,
            scheme: d.scheme,
            subdomain: d.host.subdomain,
            domain: d.host.domain,
            rest: d.rest,
            punycode: d.host.punycode,
        },
    )
}

/// The address a user sees and edits: `scyde://settings` instead of the protocol URL.
pub fn user_url(url: &str) -> String {
    internal::page_path(url).map_or_else(
        || url.to_owned(),
        |p| format!("scyde://{}", p.trim_matches('/')),
    )
}

pub fn emit_state(app: &AppHandle) {
    let state = app.state::<AppState>();
    let tabs = lock(&state.tabs);
    let list: Vec<TabView> = tabs
        .list
        .iter()
        .map(|t| TabView {
            id: t.id,
            title: t.title.clone(),
            url: user_url(&t.url),
            display: display_for(&t.url),
            loading: t.loading,
            popups_blocked: t.popups_blocked,
        })
        .collect();
    let appearance = lock(&state.settings)
        .get("appearance")
        .cloned()
        .unwrap_or(Value::Null);
    let _ = app.emit_to(
        "chrome",
        "scyde://state",
        json!({ "tabs": list, "active": tabs.active, "appearance": appearance, "filters": state.filters_version }),
    );
}

// ------------------------------------------------------------------------------------- tabs

fn content_bounds(window: &Window) -> (LogicalPosition<f64>, LogicalSize<f64>) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window
        .inner_size()
        .map(|s| s.to_logical::<f64>(scale))
        .unwrap_or(LogicalSize::new(START_W, START_H));
    // The page fills the whole window; the island floats over it in its own window (ADR-0014).
    (LogicalPosition::new(0.0, 0.0), size)
}

pub fn relayout(app: &AppHandle) {
    let Some(window) = app.get_window("main") else {
        return;
    };
    let (pos, content) = content_bounds(&window);
    let state = app.state::<AppState>();
    for t in &lock(&state.tabs).list {
        let _ = t.webview.set_position(pos);
        let _ = t.webview.set_size(content);
    }
    place_island(app);
}

/// Keeps the island window centred over the bottom of the main window's client area (screen
/// coordinates, physical pixels). Its own window is transparent, so the page shows around it.
fn place_island(app: &AppHandle) {
    let (Some(main), Some(island)) = (app.get_window("main"), app.get_webview_window("chrome"))
    else {
        return;
    };
    let scale = main.scale_factor().unwrap_or(1.0);
    let (Ok(pos), Ok(size)) = (main.inner_position(), main.inner_size()) else {
        return;
    };
    let w = (ISLAND_W * scale).min(f64::from(size.width));
    let h = (CHROME_H * scale).min(f64::from(size.height));
    let x = f64::from(pos.x) + (f64::from(size.width) - w) / 2.0;
    // Hidden = parked off-screen: unlike hide()/show(), moving it never takes focus from the page.
    let hidden = *lock(&app.state::<AppState>().island_hidden);
    let y = if hidden {
        OFFSCREEN
    } else {
        f64::from(pos.y) + f64::from(size.height) - h
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        let _ = island.set_size(PhysicalSize::new(w.round() as u32, h.round() as u32));
        let _ = island.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
    }
}

fn record_visit(app: &AppHandle, id: u32, url: &str) {
    let state = app.state::<AppState>();
    let (typed, title) = {
        let mut tabs = lock(&state.tabs);
        let Some(t) = tabs.list.iter_mut().find(|t| t.id == id) else {
            return;
        };
        (std::mem::take(&mut t.typed), t.title.clone())
    };
    if internal::page_path(url).is_some() {
        return;
    }
    let kind = if typed {
        VisitKind::Typed
    } else {
        VisitKind::Link
    };
    if let Some(p) = lock(&state.places).as_mut() {
        let _ = p.record_visit(
            url,
            (!title.is_empty()).then_some(title.as_str()),
            kind,
            now(),
        );
    }
}

pub fn new_tab(app: &AppHandle, url: Option<&str>, activate: bool) -> tauri::Result<u32> {
    let window = app.get_window("main").ok_or(tauri::Error::WindowNotFound)?;
    let state = app.state::<AppState>();
    let id = {
        let mut tabs = lock(&state.tabs);
        tabs.next_id += 1;
        tabs.next_id
    };
    let target = url.unwrap_or(NEWTAB);
    let parsed: Url = target
        .parse()
        .unwrap_or_else(|_| NEWTAB.parse().expect("newtab url"));
    let (app_load, app_title, app_nav) = (app.clone(), app.clone(), app.clone());
    let builder = env_opts(
        WebviewBuilder::new(format!("tab-{id}"), WebviewUrl::External(parsed)),
        &state,
    )
    // De-AMP: a Google AMP viewer URL loads the publisher's page instead (rule 6: real domain).
    .on_navigation(move |url| {
        let Some(target) = omnibox::deamp(url.as_str()).and_then(|t| t.parse::<Url>().ok()) else {
            return true;
        };
        let app = app_nav.clone();
        let _ = app_nav.run_on_main_thread(move || {
            if let Some(wv) = app.get_webview(&format!("tab-{id}")) {
                let _ = wv.navigate(target);
            }
        });
        false
    })
    .on_page_load(move |wv, payload| {
        let url = payload.url().to_string();
        let started = matches!(payload.event(), PageLoadEvent::Started);
        {
            let st = app_load.state::<AppState>();
            let mut tabs = lock(&st.tabs);
            if let Some(t) = tabs
                .list
                .iter_mut()
                .find(|t| t.webview.label() == wv.label())
            {
                t.url = url.clone();
                t.loading = started;
            }
        }
        if !started {
            record_visit(&app_load, id, &url);
        }
        emit_state(&app_load);
    })
    .on_document_title_changed(move |wv, title| {
        let st = app_title.state::<AppState>();
        let url = {
            let mut tabs = lock(&st.tabs);
            let Some(t) = tabs
                .list
                .iter_mut()
                .find(|t| t.webview.label() == wv.label())
            else {
                return;
            };
            t.title = title.clone();
            t.url.clone()
        };
        if internal::page_path(&url).is_none()
            && let Some(p) = lock(&st.places).as_ref()
        {
            let _ = p.set_title(&url, &title);
        }
        emit_state(&app_title);
    });
    let (pos, size) = content_bounds(&window);
    let webview = window.add_child(builder, pos, size)?;
    #[cfg(windows)]
    win::install(app, &webview, id);
    {
        let mut tabs = lock(&state.tabs);
        tabs.list.push(Tab {
            id,
            webview,
            url: target.to_owned(),
            title: String::new(),
            loading: true,
            popups_blocked: 0,
            typed: false,
        });
    }
    if activate {
        switch_tab(app, id);
    } else if let Some(t) = lock(&state.tabs).list.iter().find(|t| t.id == id) {
        let _ = t.webview.hide();
    }
    emit_state(app);
    Ok(id)
}

pub fn switch_tab(app: &AppHandle, id: u32) {
    let state = app.state::<AppState>();
    {
        let mut tabs = lock(&state.tabs);
        if !tabs.list.iter().any(|t| t.id == id) {
            return;
        }
        for t in &tabs.list {
            if t.id == id {
                let _ = t.webview.show();
                let _ = t.webview.set_focus();
            } else {
                let _ = t.webview.hide();
            }
        }
        tabs.active = Some(id);
    }
    emit_state(app);
}

pub fn close_tab(app: &AppHandle, id: u32) {
    let state = app.state::<AppState>();
    let next = {
        let mut tabs = lock(&state.tabs);
        let Some(i) = tabs.list.iter().position(|t| t.id == id) else {
            return;
        };
        let t = tabs.list.remove(i);
        if internal::page_path(&t.url).is_none() {
            tabs.closed.push(t.url.clone());
        }
        let _ = t.webview.close();
        if tabs.active == Some(id) {
            tabs.active = None;
            tabs.list.get(i).or_else(|| tabs.list.last()).map(|t| t.id)
        } else {
            tabs.active
        }
    };
    match next {
        Some(n) => switch_tab(app, n),
        None => {
            let _ = new_tab(app, None, true); // never leave the window without a tab
        }
    }
    emit_state(app);
}

pub fn cycle_tab(app: &AppHandle, delta: i32) {
    let state = app.state::<AppState>();
    let target = {
        let tabs = lock(&state.tabs);
        let n = tabs.list.len();
        if n < 2 {
            return;
        }
        let i = tabs
            .list
            .iter()
            .position(|t| Some(t.id) == tabs.active)
            .unwrap_or(0);
        let n_i = i32::try_from(n).unwrap_or(1);
        let j =
            usize::try_from((i32::try_from(i).unwrap_or(0) + delta).rem_euclid(n_i)).unwrap_or(0);
        tabs.list[j].id
    };
    switch_tab(app, target);
}

pub fn reopen_closed(app: &AppHandle) {
    let state = app.state::<AppState>();
    let url = lock(&state.tabs).closed.pop();
    if let Some(u) = url {
        let _ = new_tab(app, Some(&u), true);
    }
}

fn active_webview(app: &AppHandle) -> Option<(u32, Webview)> {
    let state = app.state::<AppState>();
    let tabs = lock(&state.tabs);
    tabs.active
        .and_then(|id| tabs.list.iter().find(|t| t.id == id))
        .map(|t| (t.id, t.webview.clone()))
}

/// Omnibox input → URL (core/omnibox) or search (settings template), in the active tab.
pub fn go(app: &AppHandle, input: &str, new_tab_too: bool) {
    let state = app.state::<AppState>();
    let url = match omnibox::classify(input) {
        omnibox::Input::Url(u) => internal::to_protocol_url(&u).unwrap_or(u),
        omnibox::Input::Search(q) => {
            omnibox::search_url(&lock(&state.settings).search_template(), &q)
        }
        omnibox::Input::Empty => return,
    };
    if new_tab_too {
        let _ = new_tab(app, Some(&url), true);
        return;
    }
    if let Some((id, wv)) = active_webview(app) {
        if let Ok(u) = url.parse() {
            if let Some(t) = lock(&state.tabs).list.iter_mut().find(|t| t.id == id) {
                t.typed = true;
            }
            let _ = wv.navigate(u);
            let _ = wv.set_focus();
        }
    } else {
        let _ = new_tab(app, Some(&url), true);
    }
}

pub fn nav(app: &AppHandle, action: &str) {
    if let Some((_, wv)) = active_webview(app) {
        let _ = match action {
            "back" => wv.eval("history.back()"),
            "forward" => wv.eval("history.forward()"),
            "reload" => wv.reload(),
            "stop" => wv.eval("window.stop()"),
            _ => Ok(()),
        };
    }
}

fn set_island_hidden(app: &AppHandle, hidden: bool) {
    let state = app.state::<AppState>();
    let changed = {
        let mut h = lock(&state.island_hidden);
        let changed = *h != hidden;
        *h = hidden;
        changed
    };
    if changed {
        place_island(app);
    }
}

/// Is the mouse over the bottom strip of the main window (where the island lives)?
fn cursor_near_island(app: &AppHandle) -> bool {
    let (Some(main), Ok(cursor)) = (app.get_window("main"), app.cursor_position()) else {
        return false;
    };
    let (Ok(pos), Ok(size)) = (main.inner_position(), main.inner_size()) else {
        return false;
    };
    let scale = main.scale_factor().unwrap_or(1.0);
    let (left, right) = (f64::from(pos.x), f64::from(pos.x) + f64::from(size.width));
    let bottom = f64::from(pos.y) + f64::from(size.height);
    cursor.x >= left
        && cursor.x <= right
        && cursor.y <= bottom
        && cursor.y >= bottom - CHROME_H * scale
}

/// Hide while reading (DESIGN v2.1): four times a second, compare the active page's scroll position
/// with the last one. Scrolling down hides the island; scrolling up, the top or end of the page,
/// the mouse near the bottom edge, or focus in the island bring it back.
#[cfg(windows)]
fn watch_island(app: AppHandle) {
    const STEP: f64 = 24.0;
    std::thread::spawn(move || {
        let mut prev: Option<(u32, f64)> = None;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(250));
            let state = app.state::<AppState>();
            let active = {
                let tabs = lock(&state.tabs);
                tabs.active.and_then(|id| {
                    tabs.list
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| (id, t.webview.clone(), internal::page_path(&t.url).is_some()))
                })
            };
            let Some((id, webview, internal)) = active else {
                continue;
            };
            let sample = lock(&state.scroll).take();
            let mut hidden = *lock(&state.island_hidden);
            if internal {
                hidden = false;
                prev = None;
            } else if let Some((sid, y, end)) = sample
                && sid == id
            {
                match prev {
                    Some((pid, py)) if pid == id => {
                        let delta = y - py;
                        if y <= 8.0 || end || delta < -STEP {
                            hidden = false;
                        } else if delta > STEP && y > 64.0 {
                            hidden = true;
                        }
                        // Small moves keep the reference point, so slow scrolling still counts.
                        if delta.abs() > STEP || y <= 8.0 {
                            prev = Some((id, y));
                        }
                    }
                    _ => {
                        prev = Some((id, y));
                        hidden = false;
                    }
                }
            }
            let focused = app
                .get_webview_window("chrome")
                .and_then(|w| w.is_focused().ok())
                .unwrap_or(false);
            if focused || cursor_near_island(&app) {
                hidden = false;
            }
            set_island_hidden(&app, hidden);
            let app_cb = app.clone();
            win::read_scroll(&webview, move |y, end| {
                *lock(&app_cb.state::<AppState>().scroll) = Some((id, y, end));
            });
        }
    });
}

pub fn focus_address(app: &AppHandle) {
    set_island_hidden(app, false);
    if let Some(window) = app.get_webview_window("chrome") {
        let _ = window.set_focus();
    }
    if let Some(chrome) = app.get_webview("chrome") {
        let _ = chrome.set_focus();
        let _ = app.emit_to("chrome", "scyde://focus-address", ());
    }
}

// ----------------------------------------------------------------------------- chrome commands

fn from_chrome(wv: &Webview) -> Result<(), String> {
    if wv.label() == "chrome" {
        Ok(())
    } else {
        Err("not allowed".into())
    }
}

#[tauri::command]
fn chrome_ready(webview: Webview, app: AppHandle) -> Result<(), String> {
    from_chrome(&webview)?;
    emit_state(&app);
    Ok(())
}

#[tauri::command]
fn chrome_action(
    webview: Webview,
    app: AppHandle,
    action: String,
    id: Option<u32>,
    input: Option<String>,
) -> Result<(), String> {
    from_chrome(&webview)?;
    // Run off the IPC thread: Tauri marshals webview calls to the main thread.
    std::thread::spawn(move || match action.as_str() {
        "new" => drop(new_tab(
            &app,
            input
                .as_deref()
                .and_then(internal::to_protocol_url)
                .as_deref(),
            true,
        )),
        "close" => id.into_iter().for_each(|i| close_tab(&app, i)),
        "switch" => id.into_iter().for_each(|i| switch_tab(&app, i)),
        "go" => go(&app, input.as_deref().unwrap_or_default(), false),
        "go-new" => go(&app, input.as_deref().unwrap_or_default(), true),
        "back" | "forward" | "reload" | "stop" => nav(&app, &action),
        "open" => {
            let page = input.unwrap_or_else(|| "newtab".into());
            if internal::PAGES.contains(&page.as_str()) {
                let _ = new_tab(&app, Some(&format!("http://scyde.localhost/{page}/")), true);
            }
        }
        "focus-page" => {
            if let Some((_, wv)) = active_webview(&app) {
                let _ = wv.set_focus();
            }
        }
        _ => {}
    });
    Ok(())
}

/// Inline completion for the address (host prefix from history, core/places).
#[tauri::command]
fn chrome_suggest(
    webview: Webview,
    app: AppHandle,
    input: String,
) -> Result<Option<String>, String> {
    from_chrome(&webview)?;
    let state = app.state::<AppState>();
    let places = lock(&state.places);
    let Some(p) = places.as_ref() else {
        return Ok(None);
    };
    let ranked = p.suggest(&input, 8).map_err(|e| e.to_string())?;
    let cands: Vec<omnibox::frecency::Candidate> = ranked
        .iter()
        .map(|s| omnibox::frecency::Candidate {
            url: s.url.clone(),
            title: s.title.clone(),
            frecency: 0.0,
        })
        .collect();
    let pairs: Vec<_> = cands
        .into_iter()
        .zip(ranked.iter().map(|s| s.quality))
        .collect();
    Ok(omnibox::frecency::inline_completion(&input, &pairs))
}

// ------------------------------------------------------------------------------------- main

fn load_settings(path: &PathBuf) -> settings::Settings {
    std::fs::read_to_string(path).map_or_else(
        |_| settings::Settings::default(),
        |s| settings::Settings::load(&s).0,
    )
}

pub fn save_settings(app: &AppHandle) {
    let state = app.state::<AppState>();
    let json = lock(&state.settings).to_json();
    let _ = std::fs::write(&state.settings_path, json);
}

fn build_window(app: &AppHandle) -> tauri::Result<()> {
    let window = WindowBuilder::new(app, "main")
        .title("SCYDE")
        .inner_size(START_W, START_H)
        .min_inner_size(560.0, 420.0)
        .build()?;
    // The island floats over the page (ADR-0014): a transparent, borderless window owned by the
    // main window, so it stays above it, hides with it when minimized and closes with it.
    let island = WebviewWindowBuilder::new(app, "chrome", WebviewUrl::App("index.html".into()))
        .title("SCYDE island")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .focused(false)
        .inner_size(ISLAND_W.min(START_W), CHROME_H);
    // Same WebView2 environment options as every tab (ADR-0003, ADR-0007).
    let state = app.state::<AppState>();
    let island = match &state.extensions {
        Some(dir) => island.browser_extensions_enabled(true).extensions_path(dir),
        None => island,
    };
    #[cfg(windows)]
    let island = island.owner_raw(window.hwnd()?);
    let island = island.build()?;
    // Alt+F4 on the island closes the browser, not just the island.
    let app_close = app.clone();
    island.on_window_event(move |e| {
        if let WindowEvent::CloseRequested { api, .. } = e {
            api.prevent_close();
            if let Some(main) = app_close.get_window("main") {
                let _ = main.close();
            }
        }
    });
    place_island(app);
    #[cfg(windows)]
    watch_island(app.clone());
    let app_ev = app.clone();
    window.on_window_event(move |e| match e {
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => relayout(&app_ev),
        WindowEvent::Moved(_) => place_island(&app_ev),
        _ => {}
    });
    #[cfg(windows)]
    if let Some(c) = app.get_webview("chrome") {
        win::install_keys(app, &c);
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    SMOKE.get_or_init(|| arg("--smoke").map(PathBuf::from));
    let open_urls: Vec<String> = args
        .iter()
        .skip(1)
        .filter(|a| a.starts_with("http://") || a.starts_with("https://"))
        .cloned()
        .collect();

    tauri::Builder::default()
        .register_uri_scheme_protocol("scyde", internal::serve)
        .invoke_handler(tauri::generate_handler![
            chrome_ready,
            chrome_action,
            chrome_suggest,
            internal::bridge
        ])
        .setup(move |app| {
            let data = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let resources = app.path().resource_dir()?;
            let ext = resources.join("extensions");
            let extensions = ext
                .join("scyde-shield")
                .join("manifest.json")
                .exists()
                .then_some(ext.clone());
            let filters_version =
                std::fs::read_to_string(ext.join("scyde-shield").join("manifest.json"))
                    .ok()
                    .and_then(|m| serde_json::from_str::<Value>(&m).ok())
                    .and_then(|m| m["description"].as_str().map(str::to_owned))
                    .and_then(|d| {
                        d.rsplit("filters ")
                            .next()
                            .map(|v| v.trim_end_matches(')').to_owned())
                    })
                    .unwrap_or_default();
            let settings_path = data.join("settings.json");
            app.manage(AppState {
                tabs: Mutex::new(Tabs::default()),
                places: Mutex::new(places::Places::open(&data.join("places.sqlite")).ok()),
                settings: Mutex::new(load_settings(&settings_path)),
                settings_path,
                extensions,
                filters_version,
                island_hidden: Mutex::new(false),
                scroll: Mutex::new(None),
            });
            let handle = app.handle().clone();
            build_window(&handle)?;
            let urls = open_urls.clone();
            std::thread::spawn(move || {
                if urls.is_empty() {
                    // First run: the welcome page (theme, search engine, Shield) instead of New Tab.
                    let first_run = SMOKE.get().is_none_or(Option::is_none)
                        && !lock(&handle.state::<AppState>().settings)
                            .get("onboarding.completed")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false);
                    let start = first_run.then_some("http://scyde.localhost/welcome/");
                    let _ = new_tab(&handle, start, true);
                } else {
                    for u in &urls {
                        let _ = new_tab(&handle, Some(u), true);
                    }
                }
                if let Some(Some(out)) = SMOKE.get() {
                    smoke(&handle, out);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("SCYDE failed to start");
}

/// `--smoke <out.json>` (CI): opens the internal pages and the qa fixtures, waits, reports what
/// loaded (titles: the qa/adblock page writes its score into `document.title`), then exits.
fn smoke(app: &AppHandle, out: &PathBuf) {
    let started = std::time::Instant::now();
    let qa = std::env::var("SCYDE_QA").unwrap_or_else(|_| "http://127.0.0.1:8787".into());
    for page in ["settings", "shields", "library"] {
        let _ = new_tab(app, Some(&format!("http://scyde.localhost/{page}/")), false);
    }
    let _ = new_tab(app, Some(&format!("{qa}/popup/index.html")), false);
    let _ = new_tab(app, Some(&format!("{qa}/adblock/index.html")), true);
    std::thread::sleep(std::time::Duration::from_secs(12));
    let state = app.state::<AppState>();
    let tabs: Vec<Value> = lock(&state.tabs)
        .list
        .iter()
        .map(|t| json!({ "url": user_url(&t.url), "title": t.title, "popups_blocked": t.popups_blocked }))
        .collect();
    let report = json!({
        "elapsed_ms": started.elapsed().as_millis(),
        "shield_extension": state.extensions.is_some(),
        "filters": state.filters_version,
        "tabs": tabs,
    });
    let _ = std::fs::write(
        out,
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    );
    std::thread::sleep(std::time::Duration::from_secs(6)); // time for the CI screenshot
    app.exit(0);
}

#[allow(dead_code)]
fn _assert_send(_: Arc<AppState>) {}
