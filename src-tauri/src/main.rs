#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod scheduler;
mod store;
mod tray;
mod win;

pub use win::toast;

use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use scheduler::Sched;
use store::Settings;

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub sched: Mutex<Sched>,
    pub debug_interval: Option<u32>,
    pub quitting: AtomicBool,
    pub toast_label: Mutex<Option<String>>,
    pub toast_seq: AtomicU64,
}

fn startup_info(app: &AppHandle) -> Value {
    // Dev builds run from the target dir - never register those for login.
    if cfg!(debug_assertions) {
        return json!({ "supported": false, "enabled": false });
    }
    let enabled = app.autolaunch().is_enabled().unwrap_or(false);
    json!({ "supported": true, "enabled": enabled })
}

fn state_payload(app: &AppHandle) -> Value {
    let state = app.state::<AppState>();
    json!({
        "settings": state.settings.lock().unwrap().clone(),
        "status": scheduler::status(&state),
        "startup": startup_info(app),
    })
}

// Push fresh state to the settings window and the tray (tray on main thread).
pub fn emit_state(app: &AppHandle) {
    let _ = app.emit_to("settings", "state:update", state_payload(app));
    let state = app.state::<AppState>();
    let status = scheduler::status(&state);
    let accent = state.settings.lock().unwrap().accent.clone();
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || tray::update(&handle, &status, &accent));
}

#[tauri::command]
fn get_state(app: AppHandle) -> Value {
    state_payload(&app)
}

#[tauri::command]
fn set_settings(app: AppHandle, patch: Value) -> Value {
    let state = app.state::<AppState>();
    let next = {
        let mut settings = state.settings.lock().unwrap();
        *settings = store::apply_patch(&settings, &patch);
        settings.clone()
    };
    store::save(&next);
    scheduler::rearm(&app); // re-emits state + tray (incl. accent icon)
    json!({ "settings": next, "status": scheduler::status(&state) })
}

#[tauri::command]
fn set_startup(app: AppHandle, enabled: bool) -> Value {
    if !cfg!(debug_assertions) {
        let autolaunch = app.autolaunch();
        let _ = if enabled {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
    }
    startup_info(&app)
}

#[tauri::command]
fn win_minimize(app: AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.minimize();
    }
}

#[tauri::command]
fn win_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn toast_done(app: AppHandle) {
    toast::close(&app);
}

#[tauri::command]
fn toast_snooze(app: AppHandle) {
    toast::close(&app);
    scheduler::snooze(&app);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let hidden = args.iter().any(|a| a == "--hidden");
    let debug_interval = args
        .iter()
        .find_map(|a| a.strip_prefix("--debug-interval-min="))
        .and_then(|v| v.parse::<u32>().ok())
        .map(|v| v.max(1));

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            win::show_settings(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .invoke_handler(tauri::generate_handler![
            get_state,
            set_settings,
            set_startup,
            win_minimize,
            win_hide,
            toast_done,
            toast_snooze
        ])
        .on_window_event(|window, event| {
            // Closing the settings window hides it; the app lives in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    let state = window.app_handle().state::<AppState>();
                    if !state.quitting.load(Ordering::SeqCst) {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .setup(move |app| {
            app.manage(AppState {
                settings: Mutex::new(store::load()),
                sched: Mutex::new(Sched::default()),
                debug_interval,
                quitting: AtomicBool::new(false),
                toast_label: Mutex::new(None),
                toast_seq: AtomicU64::new(0),
            });

            tray::create(app.handle())?;

            // Re-assert the login item so it survives updates (idempotent).
            if !cfg!(debug_assertions) && app.autolaunch().is_enabled().unwrap_or(false) {
                let _ = app.autolaunch().enable();
            }

            scheduler::rearm(app.handle());

            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                scheduler::tick(&handle);
            });

            if !hidden {
                win::show_settings(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Standup");

    app.run(|app, event| {
        // No windows open is normal for a tray app - only quit explicitly.
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() && !app.state::<AppState>().quitting.load(Ordering::SeqCst) {
                api.prevent_exit();
            }
        }
    });
}
