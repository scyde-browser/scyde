//! SCYDE for Windows: Tauri 2 multiwebview on one WebView2 environment (PLAN §9.1, ADR-0007, ADR-0016).
//!
//! Layout: a frameless window · the `sidebar` webview (web/apps/desktop-chrome) on the left holds
//! the caption buttons, navigation, the address, pinned tabs, the tabs of the current Space and the
//! Spaces · tab webviews fill the rest (two side by side in split view).
//! Internal pages are served by the `scyde` protocol (`http://scyde.localhost/<page>/` in WebView2)
//! and reach the host only through `bridge`, which checks the origin and a method allowlist.
//! Shield = the bundled MV3 extension (DNR + cosmetics, ADR-0003) on every webview; popups that the
//! user did not initiate are blocked in `NewWindowRequested` (PLAN §5.5).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod internal;
#[cfg(windows)]
mod win;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use omnibox::frecency::VisitKind;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::webview::PageLoadEvent;
use tauri::window::WindowBuilder;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, Webview, WebviewBuilder,
    WebviewUrl, Window, WindowEvent,
};

/// The sidebar's width (`size.sidebar-width`, ADR-0016).
pub const SIDEBAR_W: f64 = 280.0;
/// The line between the two pages of a split view.
const SPLIT_GAP: f64 = 1.0;
/// With the sidebar hidden, the mouse within this distance of the left edge brings it back.
const EDGE: f64 = 6.0;
const START_W: f64 = 1280.0;
const START_H: f64 = 820.0;
pub const NEWTAB: &str = "http://scyde.localhost/newtab/";
const SPACE_COLORS: [&str; 6] = ["ember", "sky", "mint", "lavender", "periwinkle", "onyx"];
const DEFAULT_SPACE: &str = "personal";

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ------------------------------------------------------------------------------------ state

/// A Space: a named group of tabs with its own Aura colour (ADR-0013 on phones, ADR-0016 here).
/// An empty name is the default Space; the sidebar shows its localized name.
#[derive(Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub color: String,
}

pub struct Tab {
    pub id: u32,
    /// None while the tab sleeps: restored tabs get their webview when they are first opened.
    pub webview: Option<Webview>,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub popups_blocked: u32,
    /// The next committed navigation came from the omnibox (frecency: typed).
    pub typed: bool,
    /// Pinned tabs show as tiles at the top of the sidebar, in every Space.
    pub pinned: bool,
    pub space: String,
}

pub struct Tabs {
    pub list: Vec<Tab>,
    pub active: Option<u32>,
    pub next_id: u32,
    pub closed: Vec<String>,
    pub spaces: Vec<Space>,
    /// The current Space.
    pub space: String,
    /// The tab each Space showed last.
    pub last: HashMap<String, u32>,
    /// Split view: these two tabs show side by side while either is active.
    pub split: Option<(u32, u32)>,
    pub next_space: u32,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            list: Vec::new(),
            active: None,
            next_id: 0,
            closed: Vec::new(),
            spaces: vec![Space {
                id: DEFAULT_SPACE.into(),
                name: String::new(),
                color: SPACE_COLORS[0].into(),
            }],
            space: DEFAULT_SPACE.into(),
            last: HashMap::new(),
            split: None,
            next_space: 0,
        }
    }
}

impl Tabs {
    /// The tabs on screen: the active one, or both halves of its split view.
    fn shown(&self) -> Vec<u32> {
        match (self.active, self.split) {
            (Some(a), Some((l, r))) if a == l || a == r => vec![l, r],
            (Some(a), _) => vec![a],
            (None, _) => Vec::new(),
        }
    }

    /// The tabs the sidebar lists, in its order: pinned tabs, then the current Space's tabs.
    fn visible(&self) -> Vec<u32> {
        let pinned = self.list.iter().filter(|t| t.pinned);
        let own = self
            .list
            .iter()
            .filter(|t| !t.pinned && t.space == self.space);
        pinned.chain(own).map(|t| t.id).collect()
    }

    fn drop_split(&mut self, id: u32) {
        if self.split.is_some_and(|(l, r)| l == id || r == id) {
            self.split = None;
        }
    }
}

/// The sidebar is on screen when it is not collapsed, or while the mouse (peek) or the keyboard
/// (hold: the address is being edited) needs it.
#[derive(Default, Clone, Copy)]
pub struct Sidebar {
    pub collapsed: bool,
    pub peek: bool,
    pub hold: bool,
}

impl Sidebar {
    fn visible(self) -> bool {
        !self.collapsed || self.peek || self.hold
    }
}

pub struct AppState {
    pub tabs: Mutex<Tabs>,
    pub places: Mutex<Option<places::Places>>,
    pub settings: Mutex<settings::Settings>,
    pub settings_path: PathBuf,
    pub extensions: Option<PathBuf>,
    /// The Shield extension must be (re)installed into the WebView2 profile on this start: first
    /// run, or a new app or filter version. Otherwise the profile already has it.
    pub install_extension: bool,
    pub filters_version: String,
    pub sidebar: Mutex<Sidebar>,
    /// Where tabs, Spaces and the sidebar state are kept between runs (None in smoke mode).
    pub session_path: Option<PathBuf>,
    /// The session as last written, so unchanged state is not written again.
    pub session_saved: Mutex<String>,
}

static SMOKE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Every webview must share the same environment options (ADR-0003, ADR-0007).
fn env_opts(b: WebviewBuilder<tauri::Wry>, state: &AppState) -> WebviewBuilder<tauri::Wry> {
    // Tabs only enable extensions. The extension itself is installed once, by the sidebar webview
    // (build_window): installing it again with every tab reloads its rulesets, and pages that load
    // meanwhile are not blocked (seen in CI: qa/adblock 3/11).
    if state.extensions.is_some() {
        b.browser_extensions_enabled(true)
    } else {
        b
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
    pinned: bool,
    space: String,
    asleep: bool,
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
    // Asked before any lock is taken: off the main thread this waits for the main thread, which
    // may itself be waiting for the tabs lock (relayout on resize). Holding the lock here hung the
    // app (CI smoke, 2026-11-13).
    let maximized = app
        .get_window("main")
        .and_then(|w| w.is_maximized().ok())
        .unwrap_or(false);
    let state = app.state::<AppState>();
    let sidebar = *lock(&state.sidebar);
    let appearance = lock(&state.settings)
        .get("appearance")
        .cloned()
        .unwrap_or(Value::Null);
    let (payload, session) = {
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
                pinned: t.pinned,
                space: t.space.clone(),
                asleep: t.webview.is_none(),
            })
            .collect();
        let payload = json!({
            "tabs": list,
            "active": tabs.active,
            "split": tabs.split.map(|(l, r)| [l, r]),
            "spaces": tabs.spaces,
            "space": tabs.space,
            "appearance": appearance,
            "filters": state.filters_version,
            "maximized": maximized,
            "collapsed": sidebar.collapsed,
        });
        let session = json!({
            "spaces": tabs.spaces,
            "space": tabs.space,
            "collapsed": sidebar.collapsed,
            "active": tabs.active.and_then(|a| tabs.list.iter().position(|t| t.id == a)),
            "tabs": tabs.list.iter().map(|t| json!({
                "url": t.url, "title": t.title, "pinned": t.pinned, "space": t.space,
            })).collect::<Vec<_>>(),
        });
        (payload, session.to_string())
    };
    let _ = app.emit_to("sidebar", "scyde://state", payload);
    if let Some(path) = &state.session_path {
        let mut saved = lock(&state.session_saved);
        if *saved != session && std::fs::write(path, &session).is_ok() {
            *saved = session;
        }
    }
}

// ----------------------------------------------------------------------------------- layout

/// The window's client size and the sidebar's width right now (logical pixels). Asks the window,
/// so it must not be called with the tabs lock held.
fn frame(app: &AppHandle, window: &Window) -> (LogicalSize<f64>, f64) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window
        .inner_size()
        .map(|s| s.to_logical::<f64>(scale))
        .unwrap_or(LogicalSize::new(START_W, START_H));
    let side = if lock(&app.state::<AppState>().sidebar).visible() {
        SIDEBAR_W.min(size.width)
    } else {
        0.0
    };
    (size, side)
}

/// Places the sidebar and the tabs on screen: one tab fills the area beside the sidebar, a split
/// view shares it; every other tab is hidden.
pub fn relayout(app: &AppHandle) {
    let Some(window) = app.get_window("main") else {
        return;
    };
    let (size, side) = frame(app, &window);
    if let Some(sidebar) = app.get_webview("sidebar") {
        if side > 0.0 {
            let _ = sidebar.set_position(LogicalPosition::new(0.0, 0.0));
            let _ = sidebar.set_size(LogicalSize::new(side, size.height));
            let _ = sidebar.show();
        } else {
            let _ = sidebar.hide();
        }
    }
    let state = app.state::<AppState>();
    let tabs = lock(&state.tabs);
    let shown = tabs.shown();
    #[allow(clippy::cast_precision_loss)]
    let n = shown.len().max(1) as f64;
    let width = ((size.width - side - SPLIT_GAP * (n - 1.0)) / n).max(0.0);
    for t in &tabs.list {
        let Some(wv) = &t.webview else { continue };
        match shown.iter().position(|id| *id == t.id) {
            Some(i) => {
                #[allow(clippy::cast_precision_loss)]
                let x = side + (width + SPLIT_GAP) * i as f64;
                let _ = wv.set_position(LogicalPosition::new(x, 0.0));
                let _ = wv.set_size(LogicalSize::new(width, size.height));
                let _ = wv.show();
            }
            None => {
                let _ = wv.hide();
            }
        }
    }
}

// ------------------------------------------------------------------------------------- tabs

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

/// Creates the webview of tab `id`, hidden behind the current page until `relayout` places it.
fn spawn_webview(app: &AppHandle, id: u32, url: &str) -> tauri::Result<Webview> {
    let window = app.get_window("main").ok_or(tauri::Error::WindowNotFound)?;
    let state = app.state::<AppState>();
    let parsed: Url = url
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
    .on_page_load(move |_wv, payload| {
        let url = payload.url().to_string();
        let started = matches!(payload.event(), PageLoadEvent::Started);
        {
            let st = app_load.state::<AppState>();
            let mut tabs = lock(&st.tabs);
            if let Some(t) = tabs.list.iter_mut().find(|t| t.id == id) {
                t.url = url.clone();
                t.loading = started;
            }
        }
        if !started {
            record_visit(&app_load, id, &url);
        }
        emit_state(&app_load);
    })
    .on_document_title_changed(move |_wv, title| {
        let st = app_title.state::<AppState>();
        let url = {
            let mut tabs = lock(&st.tabs);
            let Some(t) = tabs.list.iter_mut().find(|t| t.id == id) else {
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
    let (size, side) = frame(app, &window);
    let webview = window.add_child(
        builder,
        LogicalPosition::new(side, 0.0),
        LogicalSize::new((size.width - side).max(0.0), size.height),
    )?;
    #[cfg(windows)]
    win::install(app, &webview, id);
    Ok(webview)
}

/// The tab's webview, created now if the tab was asleep.
fn wake(app: &AppHandle, id: u32) -> Option<Webview> {
    let state = app.state::<AppState>();
    let (url, existing) = {
        let tabs = lock(&state.tabs);
        let t = tabs.list.iter().find(|t| t.id == id)?;
        (t.url.clone(), t.webview.clone())
    };
    if existing.is_some() {
        return existing;
    }
    let webview = spawn_webview(app, id, &url).ok()?;
    let mut tabs = lock(&state.tabs);
    let t = tabs.list.iter_mut().find(|t| t.id == id)?;
    t.webview = Some(webview.clone());
    t.loading = true;
    Some(webview)
}

pub fn new_tab(app: &AppHandle, url: Option<&str>, activate: bool) -> tauri::Result<u32> {
    let state = app.state::<AppState>();
    let id = {
        let mut tabs = lock(&state.tabs);
        tabs.next_id += 1;
        tabs.next_id
    };
    let target = url.unwrap_or(NEWTAB);
    let webview = spawn_webview(app, id, target)?;
    if !activate {
        let _ = webview.hide();
    }
    {
        let mut tabs = lock(&state.tabs);
        let space = tabs.space.clone();
        tabs.list.push(Tab {
            id,
            webview: Some(webview),
            url: target.to_owned(),
            title: String::new(),
            loading: true,
            popups_blocked: 0,
            typed: false,
            pinned: false,
            space,
        });
    }
    if activate {
        switch_tab(app, id);
    } else {
        emit_state(app);
    }
    Ok(id)
}

pub fn switch_tab(app: &AppHandle, id: u32) {
    let Some(webview) = wake(app, id) else {
        return;
    };
    let state = app.state::<AppState>();
    let partner = {
        let mut tabs = lock(&state.tabs);
        let Some((pinned, space)) = tabs
            .list
            .iter()
            .find(|t| t.id == id)
            .map(|t| (t.pinned, t.space.clone()))
        else {
            return;
        };
        tabs.active = Some(id);
        // A pinned tab shows in every Space; any other tab brings its own Space with it.
        if !pinned {
            tabs.space.clone_from(&space);
        }
        let current = tabs.space.clone();
        tabs.last.insert(current, id);
        tabs.split
            .and_then(|(l, r)| (l == id).then_some(r).or((r == id).then_some(l)))
    };
    // The other half of a split view may still be asleep.
    if let Some(p) = partner {
        let _ = wake(app, p);
    }
    relayout(app);
    let _ = webview.set_focus();
    emit_state(app);
}

pub fn close_tab(app: &AppHandle, id: u32) {
    let state = app.state::<AppState>();
    let next = {
        let mut tabs = lock(&state.tabs);
        let before = tabs.visible();
        let Some(i) = tabs.list.iter().position(|t| t.id == id) else {
            return;
        };
        let t = tabs.list.remove(i);
        if internal::page_path(&t.url).is_none() {
            tabs.closed.push(t.url.clone());
        }
        if let Some(wv) = &t.webview {
            let _ = wv.close();
        }
        tabs.drop_split(id);
        tabs.last.retain(|_, v| *v != id);
        if tabs.active == Some(id) {
            tabs.active = None;
            // The neighbour in the sidebar's order: the next tab, else the previous one.
            let at = before.iter().position(|v| *v == id).unwrap_or(0);
            let after = tabs.visible();
            after
                .get(at)
                .or_else(|| after.last())
                .copied()
                .map_or(Next::New, Next::Switch)
        } else {
            Next::Keep
        }
    };
    match next {
        Next::Switch(n) => switch_tab(app, n),
        // Never leave a Space without a tab.
        Next::New => drop(new_tab(app, None, true)),
        Next::Keep => {
            relayout(app);
            emit_state(app);
        }
    }
}

enum Next {
    Switch(u32),
    New,
    Keep,
}

pub fn cycle_tab(app: &AppHandle, delta: i32) {
    let state = app.state::<AppState>();
    let target = {
        let tabs = lock(&state.tabs);
        let ids = tabs.visible();
        let n = ids.len();
        if n < 2 {
            return;
        }
        let i = ids
            .iter()
            .position(|id| Some(*id) == tabs.active)
            .unwrap_or(0);
        let n_i = i32::try_from(n).unwrap_or(1);
        let j =
            usize::try_from((i32::try_from(i).unwrap_or(0) + delta).rem_euclid(n_i)).unwrap_or(0);
        ids[j]
    };
    switch_tab(app, target);
}

/// Ctrl+1…8 opens that tab of the sidebar; Ctrl+9 the last one.
pub fn switch_nth(app: &AppHandle, n: usize) {
    let target = {
        let ids = lock(&app.state::<AppState>().tabs).visible();
        if n >= 8 {
            ids.last().copied()
        } else {
            ids.get(n).copied()
        }
    };
    if let Some(id) = target {
        switch_tab(app, id);
    }
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
        .and_then(|t| t.webview.clone().map(|w| (t.id, w)))
}

fn toggle_pin(app: &AppHandle, id: u32) {
    {
        let state = app.state::<AppState>();
        let mut tabs = lock(&state.tabs);
        let space = tabs.space.clone();
        if let Some(t) = tabs.list.iter_mut().find(|t| t.id == id) {
            t.pinned = !t.pinned;
            // An unpinned tab lands in the Space the user is looking at.
            if !t.pinned {
                t.space = space;
            }
        }
    }
    emit_state(app);
}

/// Split view: `id` opens beside the active tab; asking again for a tab of the split closes it.
fn toggle_split(app: &AppHandle, id: u32) {
    let state = app.state::<AppState>();
    let opened = {
        let mut tabs = lock(&state.tabs);
        match (tabs.active, tabs.split) {
            (_, Some((l, r))) if l == id || r == id => {
                tabs.split = None;
                false
            }
            (Some(a), _) if a != id && tabs.list.iter().any(|t| t.id == id) => {
                tabs.split = Some((a, id));
                true
            }
            _ => return,
        }
    };
    if opened {
        let _ = wake(app, id);
    }
    relayout(app);
    emit_state(app);
}

// ----------------------------------------------------------------------------------- spaces

fn switch_space(app: &AppHandle, space: &str) {
    let state = app.state::<AppState>();
    let target = {
        let mut tabs = lock(&state.tabs);
        if !tabs.spaces.iter().any(|s| s.id == space) {
            return;
        }
        tabs.space = space.to_owned();
        let own = |t: &&Tab| !t.pinned && t.space == space;
        tabs.last
            .get(space)
            .copied()
            .filter(|id| tabs.list.iter().filter(own).any(|t| t.id == *id))
            .or_else(|| tabs.list.iter().find(own).map(|t| t.id))
    };
    match target {
        Some(id) => switch_tab(app, id),
        None => drop(new_tab(app, None, true)), // a Space always has a tab
    }
}

fn new_space(app: &AppHandle, name: &str) {
    let id = {
        let state = app.state::<AppState>();
        let mut tabs = lock(&state.tabs);
        tabs.next_space += 1;
        let mut id = format!("space-{}", tabs.next_space);
        while tabs.spaces.iter().any(|s| s.id == id) {
            tabs.next_space += 1;
            id = format!("space-{}", tabs.next_space);
        }
        let color = SPACE_COLORS[tabs.spaces.len() % SPACE_COLORS.len()];
        tabs.spaces.push(Space {
            id: id.clone(),
            name: clean_name(name),
            color: color.into(),
        });
        id
    };
    switch_space(app, &id);
}

fn clean_name(name: &str) -> String {
    name.trim().chars().take(40).collect()
}

fn rename_space(app: &AppHandle, space: &str, name: &str) {
    let name = clean_name(name);
    if name.is_empty() {
        return;
    }
    if let Some(s) = lock(&app.state::<AppState>().tabs)
        .spaces
        .iter_mut()
        .find(|s| s.id == space)
    {
        s.name = name;
    }
    emit_state(app);
}

/// Deletes a Space; its tabs move to the first remaining one. The last Space cannot be deleted.
fn delete_space(app: &AppHandle, space: &str) {
    let state = app.state::<AppState>();
    let go_to = {
        let mut tabs = lock(&state.tabs);
        if tabs.spaces.len() < 2 || !tabs.spaces.iter().any(|s| s.id == space) {
            return;
        }
        tabs.spaces.retain(|s| s.id != space);
        tabs.last.remove(space);
        let home = tabs.spaces[0].id.clone();
        for t in tabs.list.iter_mut().filter(|t| t.space == space) {
            t.space.clone_from(&home);
        }
        (tabs.space == space).then_some(home)
    };
    match go_to {
        Some(home) => switch_space(app, &home),
        None => emit_state(app),
    }
}

// ---------------------------------------------------------------------------------- sidebar

fn update_sidebar(app: &AppHandle, f: impl FnOnce(&mut Sidebar)) {
    let changed = {
        let state = app.state::<AppState>();
        let mut s = lock(&state.sidebar);
        let before = (s.visible(), s.collapsed);
        f(&mut s);
        before != (s.visible(), s.collapsed)
    };
    if changed {
        relayout(app);
        emit_state(app);
    }
}

/// Ctrl+S: hide or show the sidebar (the page then fills the window).
pub fn toggle_sidebar(app: &AppHandle) {
    update_sidebar(app, |s| {
        s.collapsed = !s.collapsed;
        s.peek = false;
        s.hold = false;
    });
    if let Some((_, wv)) = active_webview(app) {
        let _ = wv.set_focus();
    }
}

/// With the sidebar hidden, the mouse at the left edge of the window brings it back for as long
/// as the mouse stays over it.
fn watch_sidebar(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(120));
            let sidebar = *lock(&app.state::<AppState>().sidebar);
            if !sidebar.collapsed {
                continue;
            }
            let (Some(main), Ok(cursor)) = (app.get_window("main"), app.cursor_position()) else {
                continue;
            };
            let (Ok(pos), Ok(size)) = (main.inner_position(), main.inner_size()) else {
                continue;
            };
            let scale = main.scale_factor().unwrap_or(1.0);
            let x = cursor.x - f64::from(pos.x);
            let y = cursor.y - f64::from(pos.y);
            let inside = y >= 0.0 && y <= f64::from(size.height) && x >= 0.0;
            if !sidebar.peek && inside && x <= EDGE * scale {
                update_sidebar(&app, |s| s.peek = true);
            } else if sidebar.peek && !(inside && x <= (SIDEBAR_W + 24.0) * scale) {
                update_sidebar(&app, |s| s.peek = false);
            }
        }
    });
}

/// Ctrl+L / Ctrl+K: the address (and command bar) in the sidebar takes the keyboard.
pub fn focus_address(app: &AppHandle) {
    update_sidebar(app, |s| s.hold = true);
    if let Some(sidebar) = app.get_webview("sidebar") {
        let _ = sidebar.set_focus();
        let _ = app.emit_to("sidebar", "scyde://focus-address", ());
    }
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

// ----------------------------------------------------------------------------- chrome commands

fn from_chrome(wv: &Webview) -> Result<(), String> {
    if wv.label() == "sidebar" {
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
    std::thread::spawn(move || {
        let text = input.as_deref().unwrap_or_default();
        match action.as_str() {
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
            "pin" => id.into_iter().for_each(|i| toggle_pin(&app, i)),
            "split" => id.into_iter().for_each(|i| toggle_split(&app, i)),
            "reopen" => reopen_closed(&app),
            "go" => go(&app, text, false),
            "go-new" => go(&app, text, true),
            "back" | "forward" | "reload" | "stop" => nav(&app, &action),
            "space" => switch_space(&app, text),
            "space-new" => new_space(&app, text),
            "space-rename" => {
                // "<space id>\n<new name>"
                if let Some((space, name)) = text.split_once('\n') {
                    rename_space(&app, space, name);
                }
            }
            "space-delete" => delete_space(&app, text),
            "sidebar" => toggle_sidebar(&app),
            // The address is being edited: keep a hidden sidebar on screen until it is done.
            "hold" => update_sidebar(&app, |s| s.hold = true),
            "release" => update_sidebar(&app, |s| s.hold = false),
            "open" => {
                // "library#bookmarks" → http://scyde.localhost/library/#bookmarks
                let (name, fragment) = text
                    .split_once('#')
                    .map_or((text, String::new()), |(n, f)| (n, format!("#{f}")));
                if internal::PAGES.contains(&name)
                    && fragment
                        .chars()
                        .all(|c| c == '#' || c.is_ascii_alphanumeric())
                {
                    let _ = new_tab(
                        &app,
                        Some(&format!("http://scyde.localhost/{name}/{fragment}")),
                        true,
                    );
                }
            }
            // Caption buttons and the drag area of the sidebar (the window has no native frame).
            "win-min" | "win-max" | "win-close" | "win-drag" => {
                if let Some(w) = app.get_window("main") {
                    let _ = match action.as_str() {
                        "win-min" => w.minimize(),
                        "win-max" if w.is_maximized().unwrap_or(false) => w.unmaximize(),
                        "win-max" => w.maximize(),
                        "win-close" => w.close(),
                        _ => w.start_dragging(),
                    };
                }
            }
            "focus-page" => {
                if let Some((_, wv)) = active_webview(&app) {
                    let _ = wv.set_focus();
                }
            }
            _ => {}
        }
    });
    Ok(())
}

#[derive(Serialize)]
struct Hit {
    url: String,
    title: String,
}

/// History matches for the command bar (core/places, local only: no network request).
#[tauri::command]
fn chrome_suggestions(webview: Webview, app: AppHandle, input: String) -> Result<Vec<Hit>, String> {
    from_chrome(&webview)?;
    let state = app.state::<AppState>();
    let places = lock(&state.places);
    let Some(p) = places.as_ref() else {
        return Ok(Vec::new());
    };
    Ok(p.suggest(&input, 6)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|s| Hit {
            url: s.url,
            title: s.title,
        })
        .collect())
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
        .min_inner_size(640.0, 420.0)
        // Frameless: the sidebar draws the caption buttons (ADR-0016). Windows keeps the shadow
        // and the resize borders.
        .decorations(false)
        .shadow(true)
        // CI screens are small: maximized, the whole window is inside the screenshot.
        .maximized(SMOKE.get().is_some_and(Option::is_some))
        .build()?;
    // The sidebar is a child webview of the main window (ADR-0016): unlike the floating island
    // window of ADR-0014 it cannot end up behind the window or off the screen. It is the first
    // webview, so it also installs the Shield extension when that is due.
    let state = app.state::<AppState>();
    let sidebar = WebviewBuilder::new("sidebar", WebviewUrl::App("index.html".into()));
    let sidebar = match &state.extensions {
        Some(dir) if state.install_extension => sidebar
            .browser_extensions_enabled(true)
            .extensions_path(dir),
        Some(_) => sidebar.browser_extensions_enabled(true),
        None => sidebar,
    };
    let (size, _) = frame(app, &window);
    let sidebar = window.add_child(
        sidebar,
        LogicalPosition::new(0.0, 0.0),
        LogicalSize::new(SIDEBAR_W, size.height),
    )?;
    // Resizing a frameless window needs Tauri's border handler, attached on this call.
    let _ = window.set_resizable(true);
    relayout(app);
    watch_sidebar(app.clone());
    let app_ev = app.clone();
    window.on_window_event(move |e| {
        if matches!(
            e,
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
        ) {
            relayout(&app_ev);
            emit_state(&app_ev); // maximized ↔ restored changes the caption button
        }
    });
    #[cfg(windows)]
    win::install_keys(app, &sidebar);
    #[cfg(not(windows))]
    let _ = sidebar;
    Ok(())
}

#[derive(Deserialize)]
struct SavedTab {
    url: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    pinned: bool,
    #[serde(default)]
    space: String,
}

#[derive(Deserialize)]
struct Session {
    #[serde(default)]
    spaces: Vec<Space>,
    #[serde(default)]
    space: String,
    #[serde(default)]
    collapsed: bool,
    active: Option<usize>,
    #[serde(default)]
    tabs: Vec<SavedTab>,
}

/// Brings back the last session: Spaces, pinned tabs and tabs. Tabs come back asleep (no webview)
/// and load when they are opened. Returns the tab to show, if there is one.
fn restore_session(app: &AppHandle) -> Option<u32> {
    let state = app.state::<AppState>();
    let text = std::fs::read_to_string(state.session_path.as_ref()?).ok()?;
    let session: Session = serde_json::from_str(&text).ok()?;
    lock(&state.sidebar).collapsed = session.collapsed;
    let mut tabs = lock(&state.tabs);
    let known = |c: &str| SPACE_COLORS.contains(&c);
    let spaces: Vec<Space> = session
        .spaces
        .into_iter()
        .filter(|s| !s.id.is_empty() && known(&s.color))
        .map(|s| Space {
            name: clean_name(&s.name),
            ..s
        })
        .collect();
    if !spaces.is_empty() {
        tabs.spaces = spaces;
    }
    let home = tabs.spaces[0].id.clone();
    tabs.space = if tabs.spaces.iter().any(|s| s.id == session.space) {
        session.space
    } else {
        home.clone()
    };
    let mut ids = Vec::new();
    for saved in session.tabs {
        // Only pages a tab may hold: the web and our own internal pages.
        if !(saved.url.starts_with("https://") || saved.url.starts_with("http://")) {
            continue;
        }
        tabs.next_id += 1;
        let id = tabs.next_id;
        let space = if tabs.spaces.iter().any(|s| s.id == saved.space) {
            saved.space
        } else {
            home.clone()
        };
        tabs.list.push(Tab {
            id,
            webview: None,
            url: saved.url,
            title: saved.title,
            loading: false,
            popups_blocked: 0,
            typed: false,
            pinned: saved.pinned,
            space,
        });
        ids.push(id);
    }
    session
        .active
        .and_then(|i| ids.get(i))
        .or_else(|| ids.first())
        .copied()
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
    let smoke_mode = SMOKE.get().is_some_and(Option::is_some);
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
            chrome_suggestions,
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
            // WebView2 keeps an installed extension in the profile, so it is installed only when
            // the app or the filters changed since the last start.
            let marker = data.join("shield-extension.version");
            let stamp = format!("{}+{filters_version}", env!("CARGO_PKG_VERSION"));
            let install_extension = extensions.is_some()
                && std::fs::read_to_string(&marker).ok().as_deref() != Some(stamp.as_str());
            app.manage(AppState {
                tabs: Mutex::new(Tabs::default()),
                places: Mutex::new(places::Places::open(&data.join("places.sqlite")).ok()),
                settings: Mutex::new(load_settings(&settings_path)),
                settings_path,
                extensions,
                install_extension,
                filters_version,
                sidebar: Mutex::new(Sidebar::default()),
                session_path: (!smoke_mode).then(|| data.join("session.json")),
                session_saved: Mutex::new(String::new()),
            });
            let handle = app.handle().clone();
            // Before the window: the session also says whether the sidebar is hidden.
            let restored = restore_session(&handle);
            build_window(&handle)?;
            let urls = open_urls.clone();
            std::thread::spawn(move || {
                // After an install, give WebView2 a moment to load the rulesets before the first
                // page, then remember the installed version.
                if install_extension {
                    std::thread::sleep(std::time::Duration::from_millis(3000));
                    let _ = std::fs::write(&marker, &stamp);
                }
                if let Some(id) = restored {
                    switch_tab(&handle, id);
                }
                if !urls.is_empty() {
                    // Links open on top of the restored session.
                    for u in &urls {
                        let _ = new_tab(&handle, Some(u), true);
                    }
                } else if restored.is_none() {
                    // First run: the welcome page (theme, search engine, Shield) instead of New Tab.
                    let first_run = !smoke_mode
                        && !lock(&handle.state::<AppState>().settings)
                            .get("onboarding.completed")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false);
                    let start = first_run.then_some("http://scyde.localhost/welcome/");
                    let _ = new_tab(&handle, start, true);
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
    // Where the sidebar and the page are, so CI can tell "not shown" from "not captured".
    let bounds = |w: Option<Webview>| {
        w.map(|w| json!({ "position": w.position().ok().map(|p| [p.x, p.y]), "size": w.size().ok().map(|s| [s.width, s.height]) }))
    };
    let sidebar = bounds(app.get_webview("sidebar"));
    let page = bounds(active_webview(app).map(|(_, w)| w));
    let main = app.get_window("main").map(|w| {
        json!({ "position": w.inner_position().ok().map(|p| [p.x, p.y]), "size": w.inner_size().ok().map(|s| [s.width, s.height]) })
    });
    let state = app.state::<AppState>();
    let tabs: Vec<Value> = lock(&state.tabs)
        .list
        .iter()
        .map(|t| json!({ "url": user_url(&t.url), "title": t.title, "popups_blocked": t.popups_blocked }))
        .collect();
    let report = json!({
        "sidebar": sidebar,
        "page": page,
        "main": main,
        "elapsed_ms": started.elapsed().as_millis(),
        "shield_extension": state.extensions.is_some(),
        "shield_installed_now": state.install_extension,
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
