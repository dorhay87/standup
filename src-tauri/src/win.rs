use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::AppState;

const TOAST_W: f64 = 404.0; // 356px card + 24px transparent margin (shadow)
const TOAST_H: f64 = 236.0;

pub fn show_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    // Created hidden; the renderer shows itself after the first render
    // (standupShowWindow) so there is no unstyled flash.
    let _ = WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("settings/index.html".into()),
    )
    .title("Standup")
    .inner_size(380.0, 648.0)
    .resizable(false)
    .maximizable(false)
    .decorations(false)
    .visible(false)
    .build();
}

fn work_area_bottom_right(app: &AppHandle) -> (f64, f64) {
    if let Ok(Some(m)) = app.primary_monitor() {
        let sf = m.scale_factor();
        let wa = m.work_area();
        let right = (wa.position.x as f64 + wa.size.width as f64) / sf;
        let bottom = (wa.position.y as f64 + wa.size.height as f64) / sf;
        return (right - TOAST_W - 2.0, bottom - TOAST_H - 2.0);
    }
    (100.0, 100.0)
}

pub mod toast {
    use super::*;

    // Refresh (close + recreate) rather than stack; labels rotate because a
    // destroyed label may not be reusable immediately.
    pub fn show(app: &AppHandle) {
        let app = app.clone();
        let _ = app.clone().run_on_main_thread(move || {
            close(&app);
            let state = app.state::<AppState>();
            let s = state.settings.lock().unwrap().clone();
            let seq = state
                .toast_seq
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let label = format!("toast-{seq}");
            let (x, y) = work_area_bottom_right(&app);
            let qs = format!("?sound={}&theme={}&accent={}", s.sound, s.theme, s.accent);
            let built =
                WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("toast/toast.html".into()))
                    .initialization_script(format!("window.__TOAST_QS__ = '{qs}';"))
                    .title("Standup reminder")
                    .inner_size(TOAST_W, TOAST_H)
                    .position(x, y)
                    .decorations(false)
                    .transparent(true)
                    .shadow(false)
                    .resizable(false)
                    .focusable(false) // never steals focus
                    .skip_taskbar(true)
                    .always_on_top(true)
                    .visible(false) // shows itself once loaded, sound plays on load
                    .additional_browser_args(
                        "--autoplay-policy=no-user-gesture-required \
                 --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection",
                    )
                    .build();
            if built.is_ok() {
                *state.toast_label.lock().unwrap() = Some(label);
            }
        });
    }

    pub fn close(app: &AppHandle) {
        let state = app.state::<AppState>();
        let label = state.toast_label.lock().unwrap().take();
        if let Some(label) = label {
            if let Some(w) = app.get_webview_window(&label) {
                let _ = w.destroy();
            }
        }
    }
}
