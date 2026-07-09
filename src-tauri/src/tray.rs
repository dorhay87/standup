use chrono::{DateTime, Local, Timelike};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::{scheduler, scheduler::Status, win, AppState};

const TRAY_ID: &str = "main";

// Pre-tinted per accent (hue-shifted copies of the base teal tray-32.png).
fn icon_for(accent: &str) -> Image<'static> {
    let bytes: &[u8] = match accent {
        "indigo" => include_bytes!("../../assets/tray-32-indigo.png"),
        "violet" => include_bytes!("../../assets/tray-32-violet.png"),
        "amber" => include_bytes!("../../assets/tray-32-amber.png"),
        "rose" => include_bytes!("../../assets/tray-32-rose.png"),
        _ => include_bytes!("../../assets/tray-32.png"),
    };
    Image::from_bytes(bytes).expect("bundled tray icon is valid")
}

fn fmt_hm(ms: i64) -> String {
    match DateTime::from_timestamp_millis(ms) {
        Some(t) => {
            let t = t.with_timezone(&Local);
            format!("{:02}:{:02}", t.hour(), t.minute())
        }
        None => "-".into(),
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let accent = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .accent
        .clone();
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon_for(&accent))
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                win::show_settings(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "pause" => scheduler::toggle_pause(app),
            "open" => win::show_settings(app),
            "quit" => {
                app.state::<AppState>()
                    .quitting
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

// Must run on the main thread.
pub fn update(app: &AppHandle, status: &Status, accent: &str) {
    let Some(tray): Option<TrayIcon> = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let now = Local::now().timestamp_millis();
    let paused = matches!(status.paused_until, Some(p) if p > now);
    let tip = if !status.enabled {
        "Standup - Off".to_string()
    } else if paused {
        format!(
            "Standup - Paused until {}",
            fmt_hm(status.paused_until.unwrap())
        )
    } else if let Some(next) = status.next_fire_at {
        format!("Standup - Next: {}", fmt_hm(next))
    } else {
        "Standup - No active days".to_string()
    };
    let _ = tray.set_tooltip(Some(&tip));
    let _ = tray.set_icon(Some(icon_for(accent)));

    if let Ok(menu) = build_menu(app, paused, status.enabled) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn build_menu(app: &AppHandle, paused: bool, enabled: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let pause_label = if paused {
        "Resume reminders"
    } else {
        "Pause for 1 hour"
    };
    let pause = MenuItem::with_id(app, "pause", pause_label, enabled, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open settings", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    Menu::with_items(app, &[&pause, &open, &sep, &quit])
}
