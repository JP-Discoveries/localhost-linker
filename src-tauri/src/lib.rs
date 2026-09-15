mod ip;
mod qr;
mod scanner;

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use scanner::Snapshot;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewWindow, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_positioner::{Position, WindowExt};

const TRAY_ID: &str = "tray";
const POLL_EVERY: Duration = Duration::from_secs(2);
/// A tray click that lands right after the popup lost focus is the user closing it, not reopening.
const REOPEN_GRACE: Duration = Duration::from_millis(250);

struct AppState {
    snapshot: Mutex<Snapshot>,
    last_hidden: Mutex<Option<Instant>>,
    autostart_item: Mutex<Option<CheckMenuItem<Wry>>>,
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Snapshot {
    state.snapshot.lock().unwrap().clone()
}

#[tauri::command]
fn copy_url(app: AppHandle, url: String) -> Result<(), String> {
    app.clipboard().write_text(url).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    apply_autostart(&app, enabled)
}

#[tauri::command]
fn hide_popup(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        hide(&app, &w);
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    let actual = launcher.is_enabled().unwrap_or(false);
    if let Some(item) = app.state::<AppState>().autostart_item.lock().unwrap().as_ref() {
        let _ = item.set_checked(actual);
    }
    let _ = app.emit("autostart-changed", actual);
    result.map(|_| actual).map_err(|e| e.to_string())
}

fn hide(app: &AppHandle, window: &WebviewWindow) {
    let _ = window.hide();
    *app.state::<AppState>().last_hidden.lock().unwrap() = Some(Instant::now());
}

fn show_popup(app: &AppHandle, near_tray: bool) {
    let Some(window) = app.get_webview_window("main") else { return };
    let latest = app.state::<AppState>().snapshot.lock().unwrap().clone();
    let _ = app.emit("servers-changed", &latest);
    let placed = near_tray && window.move_window_constrained(Position::TrayBottomCenter).is_ok();
    if !placed {
        let _ = window.move_window(Position::BottomRight);
    }
    let _ = window.show();
    let _ = window.set_focus();
}

fn toggle_popup_from_tray(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    if window.is_visible().unwrap_or(false) {
        hide(app, &window);
        return;
    }
    let just_hidden = app
        .state::<AppState>()
        .last_hidden
        .lock()
        .unwrap()
        .is_some_and(|t| t.elapsed() < REOPEN_GRACE);
    if !just_hidden {
        show_popup(app, true);
    }
}

fn tooltip_for(snap: &Snapshot) -> String {
    let visible: Vec<_> = snap.servers.iter().filter(|s| !s.hidden).collect();
    let reachable = visible.iter().filter(|s| s.reachable_from_lan).count();
    let ip = snap.lan_ip.as_deref().unwrap_or("no network");
    match visible.len() {
        0 => format!("Localhost Linker · no dev servers · {ip}"),
        n => format!(
            "Localhost Linker · {n} server{} ({reachable} on LAN) · {ip}",
            if n == 1 { "" } else { "s" }
        ),
    }
}

fn start_polling(app: AppHandle) {
    thread::spawn(move || {
        let mut last_fingerprint: Option<u64> = None;
        let mut last_error: Option<String> = None;
        loop {
            let lan_ip = ip::lan_ipv4();
            match scanner::read_sockets() {
                Ok(sockets) => {
                    let fp = scanner::fingerprint(&sockets, lan_ip);
                    if last_fingerprint != Some(fp) {
                        last_fingerprint = Some(fp);
                        last_error = None;
                        publish(
                            &app,
                            Snapshot {
                                lan_ip: lan_ip.map(|ip| ip.to_string()),
                                servers: scanner::build(&sockets, lan_ip),
                                scanned_at: scanner::now_secs(),
                                error: None,
                            },
                        );
                    }
                }
                Err(err) => {
                    if last_error.as_ref() != Some(&err) {
                        last_error = Some(err.clone());
                        last_fingerprint = None;
                        publish(
                            &app,
                            Snapshot {
                                lan_ip: lan_ip.map(|ip| ip.to_string()),
                                error: Some(err),
                                scanned_at: scanner::now_secs(),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
            thread::sleep(POLL_EVERY);
        }
    });
}

fn publish(app: &AppHandle, snap: Snapshot) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tooltip_for(&snap)));
    }
    // A hidden popup doesn't need live updates: every event costs WebView2 memory it is slow to
    // give back. show_popup() pushes the latest snapshot right before the window appears.
    // Store before checking visibility, so a popup opening mid-publish still gets this snapshot.
    *app.state::<AppState>().snapshot.lock().unwrap() = snap.clone();
    let visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    if visible {
        let _ = app.emit("servers-changed", &snap);
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let handle = app.handle();
    let open = MenuItem::with_id(handle, "open", "Show servers", true, None::<&str>)?;
    let autostart_on = handle.autolaunch().is_enabled().unwrap_or(false);
    let autostart =
        CheckMenuItem::with_id(handle, "autostart", "Launch at login", true, autostart_on, None::<&str>)?;
    let quit = MenuItem::with_id(handle, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        handle,
        &[
            &open,
            &PredefinedMenuItem::separator(handle)?,
            &autostart,
            &PredefinedMenuItem::separator(handle)?,
            &quit,
        ],
    )?;
    *app.state::<AppState>().autostart_item.lock().unwrap() = Some(autostart.clone());

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Localhost Linker · scanning…")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_popup(app, false),
            "autostart" => {
                let want = !app.autolaunch().is_enabled().unwrap_or(false);
                let _ = apply_autostart(app, want);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_popup_from_tray(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_popup(app, false)))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .manage(AppState {
            snapshot: Mutex::new(Snapshot::default()),
            last_hidden: Mutex::new(None),
            autostart_item: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            copy_url,
            get_autostart,
            set_autostart,
            hide_popup,
            quit
        ])
        .setup(|app| {
            build_tray(app)?;
            start_polling(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::Focused(false) => {
                if let Some(w) = window.app_handle().get_webview_window(window.label()) {
                    hide(window.app_handle(), &w);
                }
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("failed to build Localhost Linker");

    app.run(|_app, event| {
        // Closing the popup must not quit a tray app; only an explicit app.exit() passes through.
        if let RunEvent::ExitRequested { code: None, api, .. } = event {
            api.prevent_exit();
        }
    });
}
