//! Spikes on WebView2 via Tauri 2 multiwebview (feature `unstable`):
//! - S5 / G5 (`--mode bench`): 20 tab webviews on one environment, tab switch and popover timings, Mica, custom titlebar.
//! - S2 / G2 (`--mode shield --shield none|b`): page load times with no blocking vs option B
//!   (`WebResourceRequested` + adblock-rust in the host), plus the qa/adblock fixture score.
//! - S1 / G1 (`--mode signin`): opens Google sign-in, Gmail and YouTube with the stock WebView2 UA for a manual test.
//! Results are written as JSON to `--out` (default `spike-results.json`) and the app exits by itself in bench/shield modes.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Webview, WebviewBuilder, WebviewUrl, Window,
    webview::PageLoadEvent, window::WindowBuilder,
};

const W: f64 = 1280.0;
const H: f64 = 800.0;
const CHROME_H: f64 = 92.0; // tab strip 44 + toolbar 48 (DESIGN §4)

#[derive(Default)]
struct Tabs {
    views: Vec<Webview>,
    titles: Vec<String>,
    active: usize,
}

#[derive(Clone, Serialize)]
struct TabsEvent {
    titles: Vec<String>,
    active: usize,
    url: String,
}

#[derive(Default)]
struct Loads {
    /// label -> (started, url)
    started: HashMap<String, (Instant, String)>,
    /// finished loads in order: (url, ms)
    done: Vec<(String, f64)>,
    titles: HashMap<String, String>,
}

struct AppState {
    tabs: Mutex<Tabs>,
    loads: Arc<Mutex<Loads>>,
    blocked: Arc<Mutex<u64>>,
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn percentile(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = ((p * v.len() as f64).floor() as usize).min(v.len() - 1);
    v[idx]
}

fn emit_tabs(app: &AppHandle) {
    let state = app.state::<AppState>();
    let tabs = state.tabs.lock().unwrap();
    let url = tabs.views.get(tabs.active).and_then(|w| w.url().ok()).map(|u| u.to_string()).unwrap_or_default();
    let _ = app.emit_to("chrome", "tabs", TabsEvent { titles: tabs.titles.clone(), active: tabs.active, url });
}

fn tab_builder(label: String, url: &str, loads: Arc<Mutex<Loads>>) -> WebviewBuilder<tauri::Wry> {
    let l1 = loads.clone();
    let l2 = loads;
    let lbl = label.clone();
    WebviewBuilder::new(label, WebviewUrl::External(url.parse().expect("url")))
        .on_page_load(move |wv, payload| {
            let mut l = l1.lock().unwrap();
            match payload.event() {
                PageLoadEvent::Started => {
                    l.started.insert(wv.label().to_string(), (Instant::now(), payload.url().to_string()));
                }
                PageLoadEvent::Finished => {
                    if let Some((t0, u)) = l.started.remove(wv.label()) {
                        l.done.push((u, t0.elapsed().as_secs_f64() * 1000.0));
                    }
                }
            }
        })
        .on_document_title_changed(move |_wv, title| {
            l2.lock().unwrap().titles.insert(lbl.clone(), title);
        })
}

/// Switch = hide current + show target (+ focus). Returns host-side duration in ms.
fn switch_to(app: &AppHandle, index: usize) -> f64 {
    let state = app.state::<AppState>();
    let t0 = Instant::now();
    {
        let mut tabs = state.tabs.lock().unwrap();
        if index >= tabs.views.len() || index == tabs.active {
            return 0.0;
        }
        let _ = tabs.views[index].show();
        let _ = tabs.views[tabs.active].hide();
        let _ = tabs.views[index].set_focus();
        tabs.active = index;
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    emit_tabs(app);
    ms
}

#[tauri::command]
fn switch_tab(app: AppHandle, index: usize) {
    switch_to(&app, index);
}
#[tauri::command]
fn win_minimize(window: Window) {
    let _ = window.minimize();
}
#[tauri::command]
fn win_toggle_maximize(window: Window) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}
#[tauri::command]
fn win_close(app: AppHandle) {
    app.exit(0);
}
#[tauri::command]
fn toggle_popover(app: AppHandle) {
    if let Some(p) = app.get_webview_window("popover") {
        if p.is_visible().unwrap_or(false) {
            let _ = p.hide();
        } else {
            let _ = p.show();
        }
    }
}
#[tauri::command]
fn chrome_ready(app: AppHandle) {
    emit_tabs(&app);
}

fn build_window(app: &AppHandle) -> tauri::Result<Window> {
    let effects = tauri::utils::config::WindowEffectsConfig {
        effects: vec![tauri::window::Effect::Mica],
        ..Default::default()
    };
    let window = WindowBuilder::new(app, "main")
        .title("SCYDE Spike S5")
        .inner_size(W, H)
        .decorations(false)
        .transparent(true)
        .effects(effects)
        .build()?;
    window.add_child(
        WebviewBuilder::new("chrome", WebviewUrl::App("index.html".into())).transparent(true),
        LogicalPosition::new(0.0, 0.0),
        LogicalSize::new(W, CHROME_H),
    )?;
    Ok(window)
}

fn add_tab(app: &AppHandle, window: &Window, label: &str, url: &str, title: &str, visible: bool) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let wv = window.add_child(
        tab_builder(label.to_string(), url, state.loads.clone()),
        LogicalPosition::new(0.0, CHROME_H),
        LogicalSize::new(W, H - CHROME_H),
    )?;
    if !visible {
        wv.hide()?;
    }
    let mut tabs = state.tabs.lock().unwrap();
    tabs.views.push(wv);
    tabs.titles.push(title.to_string());
    Ok(())
}

fn write_report(path: &str, value: &serde_json::Value) {
    let s = serde_json::to_string_pretty(value).unwrap();
    println!("SCYDE_RESULT {s}");
    std::fs::write(path, s).expect("write report");
}

// ---------------- S5 / G5 ----------------
fn run_bench(app: AppHandle, out: String, qa: String) {
    let window = app.get_window("main").unwrap();
    let t0 = Instant::now();
    for i in 0..20 {
        let url = format!("{qa}/popup/target.html?tab={i}");
        add_tab(&app, &window, &format!("tab-{i}"), &url, &format!("Tab {i}"), i == 0).unwrap();
    }
    let create_ms = t0.elapsed().as_secs_f64() * 1000.0;
    emit_tabs(&app);
    std::thread::sleep(Duration::from_secs(3)); // let all tabs load

    // 200 switches in a fixed pseudo-random order
    let mut seed: u64 = 7;
    let mut switches = Vec::new();
    for _ in 0..200 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let target = (seed >> 33) as usize % 20;
        let ms = switch_to(&app, target);
        if ms > 0.0 {
            switches.push(ms);
        }
        std::thread::sleep(Duration::from_millis(16));
    }

    // Pre-created popover: show/hide latency
    let pop = app.get_webview_window("popover").unwrap();
    let mut shows = Vec::new();
    for _ in 0..50 {
        let t = Instant::now();
        pop.show().unwrap();
        shows.push(t.elapsed().as_secs_f64() * 1000.0);
        std::thread::sleep(Duration::from_millis(30));
        pop.hide().unwrap();
        std::thread::sleep(Duration::from_millis(30));
    }
    pop.show().unwrap(); // leave it visible for the screenshot

    let loads = app.state::<AppState>().loads.lock().unwrap().done.len();
    let mut s = switches.clone();
    let mut p = shows.clone();
    let report = serde_json::json!({
        "spike": "S5",
        "tabs": 20,
        "tabs_loaded": loads,
        "create_20_tabs_ms": create_ms,
        "switch_ms": { "n": s.len(), "p50": percentile(&mut s, 0.5), "p95": percentile(&mut s, 0.95), "max": percentile(&mut s, 1.0) },
        "popover_show_ms": { "n": p.len(), "p50": percentile(&mut p, 0.5), "p95": percentile(&mut p, 0.95), "max": percentile(&mut p, 1.0) },
        "g5_switch_le_16ms": percentile(&mut switches, 0.95) <= 16.0,
        "g5_popover_le_16ms": percentile(&mut shows, 0.95) <= 16.0,
        "note": "Host-side duration of hide/show/focus (and popover show). Visual frame timing is checked on the screenshot/video artefact."
    });
    write_report(&out, &report);
}

// ---------------- S2 / G2 option B ----------------
#[cfg(windows)]
mod option_b {
    use std::sync::{Arc, Mutex};
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL, ICoreWebView2, ICoreWebView2Environment,
        ICoreWebView2WebResourceRequestedEventArgs,
    };
    use webview2_com::WebResourceRequestedEventHandler;
    use windows::core::{HSTRING, PWSTR};

    fn take(p: PWSTR) -> String {
        if p.is_null() {
            return String::new();
        }
        let s = unsafe { p.to_string().unwrap_or_default() };
        unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as _)) };
        s
    }

    fn ctx_type(c: COREWEBVIEW2_WEB_RESOURCE_CONTEXT) -> &'static str {
        match c.0 {
            1 => "main_frame",
            2 => "stylesheet",
            3 => "image",
            4 => "media",
            5 => "font",
            6 => "script",
            7 => "xmlhttprequest",
            8 => "xmlhttprequest", // fetch
            9 => "other",          // text track
            10 => "other",         // event source
            11 => "websocket",
            12 => "other",         // manifest
            13 => "other",         // signed exchange
            14 => "ping",
            15 => "other",         // csp violation report
            _ => "other",
        }
    }

    pub fn install(wv: &tauri::Webview, engine: Arc<adblock::Engine>, blocked: Arc<Mutex<u64>>) {
        wv.with_webview(move |pw| unsafe {
            let controller = pw.controller();
            let core: ICoreWebView2 = controller.CoreWebView2().expect("core");
            let env: ICoreWebView2Environment = pw.environment();
            core.AddWebResourceRequestedFilter(&HSTRING::from("*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL).expect("filter");
            let mut token: i64 = 0;
            let handler = WebResourceRequestedEventHandler::create(Box::new(
                move |_sender, args: Option<ICoreWebView2WebResourceRequestedEventArgs>| {
                    let Some(args) = args else { return Ok(()) };
                    let req = args.Request()?;
                    let mut uri = PWSTR::null();
                    req.Uri(&mut uri)?;
                    let uri = take(uri);
                    let mut ctx = COREWEBVIEW2_WEB_RESOURCE_CONTEXT::default();
                    args.ResourceContext(&mut ctx)?;
                    let kind = ctx_type(ctx);
                    if kind == "main_frame" {
                        return Ok(());
                    }
                    let mut referer = PWSTR::null();
                    let source = match req.Headers().and_then(|h| h.GetHeader(&HSTRING::from("Referer"), &mut referer)) {
                        Ok(()) => take(referer),
                        Err(_) => uri.clone(),
                    };
                    if let Ok(r) = adblock::request::Request::new(&uri, &source, kind, "GET") {
                        if engine.check_network_request(&r).should_block() {
                            *blocked.lock().unwrap() += 1;
                            let resp = env.CreateWebResourceResponse(None, 403, &HSTRING::from("Blocked by SCYDE Shield"), &HSTRING::from(""))?;
                            args.SetResponse(&resp)?;
                        }
                    }
                    Ok(())
                },
            ));
            core.add_WebResourceRequested(&handler, &mut token).expect("add handler");
        })
        .expect("with_webview");
    }
}

fn build_engine(list_paths: &[String]) -> adblock::Engine {
    let mut set = adblock::lists::FilterSet::new(false);
    for p in list_paths {
        if let Ok(text) = std::fs::read_to_string(p) {
            set.add_filter_list(text, adblock::lists::ParseOptions::default());
        }
    }
    adblock::Engine::new_with_filter_set(set)
}

fn run_shield(app: AppHandle, out: String, qa: String, shield: String, sites: Vec<String>, lists: Vec<String>, rounds: usize) {
    let window = app.get_window("main").unwrap();
    let state = app.state::<AppState>();
    let engine_ms;
    let t = Instant::now();
    let engine = Arc::new(build_engine(&lists));
    engine_ms = t.elapsed().as_secs_f64() * 1000.0;

    add_tab(&app, &window, "tab-0", &format!("{qa}/popup/target.html?warmup"), "S2", true).unwrap();
    emit_tabs(&app);
    std::thread::sleep(Duration::from_secs(2));
    let tab = state.tabs.lock().unwrap().views[0].clone();
    #[cfg(windows)]
    if shield == "b" {
        option_b::install(&tab, engine.clone(), state.blocked.clone());
    }
    #[cfg(not(windows))]
    let _ = &engine;

    let mut per_site = Vec::new();
    let mut all = vec![format!("{qa}/adblock/")];
    all.extend(sites);
    for url in &all {
        let mut times = Vec::new();
        let blocked_before = *state.blocked.lock().unwrap();
        for _ in 0..rounds {
            state.loads.lock().unwrap().done.clear();
            let _ = tab.navigate(url.parse().unwrap());
            let t0 = Instant::now();
            loop {
                std::thread::sleep(Duration::from_millis(50));
                if let Some((_, ms)) = state.loads.lock().unwrap().done.last().cloned() {
                    times.push(ms);
                    break;
                }
                if t0.elapsed() > Duration::from_secs(45) {
                    times.push(45_000.0);
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(1500)); // let the fixture write its title
        }
        let title = state.loads.lock().unwrap().titles.get("tab-0").cloned().unwrap_or_default();
        let blocked = (*state.blocked.lock().unwrap() - blocked_before) / rounds as u64;
        let mut t2 = times.clone();
        per_site.push(serde_json::json!({ "url": url, "median_ms": percentile(&mut t2, 0.5), "runs_ms": times, "blocked_per_load": blocked, "title": title }));
    }
    let report = serde_json::json!({
        "spike": "S2", "option": shield, "rounds": rounds, "engine_build_ms": engine_ms, "lists": lists, "sites": per_site,
        "ua": "stock WebView2 (unchanged)",
    });
    write_report(&out, &report);
}

fn run_signin(app: AppHandle) {
    let window = app.get_window("main").unwrap();
    let urls = [
        ("https://accounts.google.com/", "Google prijava"),
        ("https://mail.google.com/", "Gmail"),
        ("https://www.youtube.com/", "YouTube"),
        ("https://drive.google.com/", "Drive"),
        ("https://stackoverflow.com/users/login", "Sign in with Google (3rd party)"),
    ];
    for (i, (u, t)) in urls.iter().enumerate() {
        add_tab(&app, &window, &format!("tab-{i}"), u, t, i == 0).unwrap();
    }
    emit_tabs(&app);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = arg(&args, "--mode").unwrap_or_else(|| "signin".into());
    let out = arg(&args, "--out").unwrap_or_else(|| "spike-results.json".into());
    let qa = arg(&args, "--qa").unwrap_or_else(|| "http://127.0.0.1:8787".into());
    let shield = arg(&args, "--shield").unwrap_or_else(|| "none".into());
    let rounds: usize = arg(&args, "--rounds").and_then(|r| r.parse().ok()).unwrap_or(3);
    let sites: Vec<String> = arg(&args, "--sites")
        .map(|s| s.split(',').filter(|x| !x.is_empty()).map(String::from).collect())
        .unwrap_or_default();
    let lists: Vec<String> = arg(&args, "--lists")
        .map(|s| s.split(',').filter(|x| !x.is_empty()).map(String::from).collect())
        .unwrap_or_default();
    let hold: u64 = arg(&args, "--hold").and_then(|h| h.parse().ok()).unwrap_or(6);

    tauri::Builder::default()
        .manage(AppState { tabs: Mutex::new(Tabs::default()), loads: Arc::new(Mutex::new(Loads::default())), blocked: Arc::new(Mutex::new(0)) })
        .invoke_handler(tauri::generate_handler![switch_tab, win_minimize, win_toggle_maximize, win_close, toggle_popover, chrome_ready])
        .setup(move |app| {
            let handle = app.handle().clone();
            build_window(&handle)?;
            // Popover: separate, pre-created, hidden transparent window (CLAUDE.md desktop notes).
            tauri::WebviewWindowBuilder::new(&handle, "popover", WebviewUrl::App("popover.html".into()))
                .transparent(true)
                .decorations(false)
                .skip_taskbar(true)
                .always_on_top(true)
                .resizable(false)
                .shadow(false)
                .inner_size(376.0, 200.0)
                .position(880.0, 120.0)
                .visible(false)
                .build()?;

            let mode = mode.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(800));
                match mode.as_str() {
                    "bench" => run_bench(handle.clone(), out, qa),
                    "shield" => run_shield(handle.clone(), out, qa, shield, sites, lists, rounds),
                    _ => {
                        run_signin(handle.clone());
                        return;
                    }
                }
                std::thread::sleep(Duration::from_secs(hold)); // time for a desktop screenshot
                handle.exit(0);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("tauri run");
}
