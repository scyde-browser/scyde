//! WebView2 COM hooks Tauri does not expose (ADR-0007 escape hatch `with_webview`):
//! - popup blocker: `NewWindowRequested` without a user gesture is cancelled (PLAN §5.5); a
//!   user-initiated popup keeps the default behaviour, so `window.opener` flows (OAuth) work.
//! - shortcuts: `AcceleratorKeyPressed` (Ctrl+T/W/L/Tab, Ctrl+Shift+T/Tab, Ctrl+R, F5, Alt+←/→),
//!   so they work while a page has focus.

use tauri::{AppHandle, Manager, Webview};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_KEY_EVENT_KIND, COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN, COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN,
    ICoreWebView2, ICoreWebView2AcceleratorKeyPressedEventArgs, ICoreWebView2NewWindowRequestedEventArgs,
};
use webview2_com::{AcceleratorKeyPressedEventHandler, NewWindowRequestedEventHandler};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_SHIFT};

use crate::{AppState, close_tab, cycle_tab, emit_state, focus_address, lock, nav, new_tab, reopen_closed};

fn down(vk: u16) -> bool {
    // SAFETY: GetKeyState has no preconditions.
    unsafe { GetKeyState(i32::from(vk)) < 0 }
}

/// Runs a host action off the WebView2 callback (Tauri marshals webview calls to the main thread;
/// doing that synchronously from inside a COM callback on the main thread could deadlock).
fn later(app: &AppHandle, f: impl FnOnce(&AppHandle) + Send + 'static) {
    let app = app.clone();
    std::thread::spawn(move || f(&app));
}

fn active(app: &AppHandle) -> Option<u32> {
    lock(&app.state::<AppState>().tabs).active
}

/// Returns true if the key was a browser shortcut (and handles it).
fn shortcut(app: &AppHandle, vk: u32, kind: COREWEBVIEW2_KEY_EVENT_KIND) -> bool {
    let ctrl = down(VK_CONTROL.0);
    let shift = down(VK_SHIFT.0);
    let alt = kind == COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN;
    match (vk, ctrl, shift, alt) {
        (0x54, true, false, false) => later(app, |a| drop(new_tab(a, None, true))), // Ctrl+T
        (0x54, true, true, false) => later(app, reopen_closed),                       // Ctrl+Shift+T
        (0x57, true, _, false) => later(app, |a| {
            if let Some(id) = active(a) {
                close_tab(a, id);
            }
        }), // Ctrl+W
        (0x4C, true, false, false) | (0x44, false, false, true) => later(app, focus_address), // Ctrl+L, Alt+D
        (0x09, true, false, false) => later(app, |a| cycle_tab(a, 1)),                    // Ctrl+Tab
        (0x09, true, true, false) => later(app, |a| cycle_tab(a, -1)),                    // Ctrl+Shift+Tab
        (0x52, true, false, false) | (0x74, false, false, false) => later(app, |a| nav(a, "reload")), // Ctrl+R, F5
        (0x25, false, false, true) => later(app, |a| nav(a, "back")),                     // Alt+Left
        (0x27, false, false, true) => later(app, |a| nav(a, "forward")),                  // Alt+Right
        _ => return false,
    }
    true
}

pub fn install_keys(app: &AppHandle, webview: &Webview) {
    let app = app.clone();
    let _ = webview.with_webview(move |pw| {
        let controller = pw.controller();
        let handler = AcceleratorKeyPressedEventHandler::create(Box::new(
            move |_sender, args: Option<ICoreWebView2AcceleratorKeyPressedEventArgs>| {
                let Some(args) = args else { return Ok(()) };
                let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
                let mut vk = 0u32;
                // SAFETY: valid event args for the duration of the callback.
                unsafe {
                    args.KeyEventKind(&mut kind)?;
                    if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && kind != COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN {
                        return Ok(());
                    }
                    args.VirtualKey(&mut vk)?;
                    if shortcut(&app, vk, kind) {
                        args.SetHandled(true)?;
                    }
                }
                Ok(())
            },
        ));
        let mut token = 0i64;
        // SAFETY: the controller outlives the registration (it lives as long as the webview).
        unsafe {
            let _ = controller.add_AcceleratorKeyPressed(&handler, &mut token);
        }
    });
}

pub fn install(app: &AppHandle, webview: &Webview, tab_id: u32) {
    install_keys(app, webview);
    let app = app.clone();
    let _ = webview.with_webview(move |pw| {
        let controller = pw.controller();
        // SAFETY: CoreWebView2 is available once the controller exists.
        let Ok(core): Result<ICoreWebView2, _> = (unsafe { controller.CoreWebView2() }) else { return };
        let handler = NewWindowRequestedEventHandler::create(Box::new(
            move |_sender, args: Option<ICoreWebView2NewWindowRequestedEventArgs>| {
                let Some(args) = args else { return Ok(()) };
                let mut user = windows_core::BOOL::default();
                // SAFETY: valid event args for the duration of the callback.
                unsafe { args.IsUserInitiated(&mut user)? };
                if !user.as_bool() {
                    unsafe { args.SetHandled(true)? };
                    later(&app, move |a| {
                        {
                            let st = a.state::<AppState>();
                            let mut tabs = lock(&st.tabs);
                            if let Some(t) = tabs.list.iter_mut().find(|t| t.id == tab_id) {
                                t.popups_blocked += 1;
                            }
                        }
                        emit_state(a);
                    });
                }
                Ok(())
            },
        ));
        let mut token = 0i64;
        // SAFETY: as above.
        unsafe {
            let _ = core.add_NewWindowRequested(&handler, &mut token);
        }
    });
}
