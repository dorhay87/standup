'use strict';
// IPC facade: the renderers talk to `window.standup` / `window.standupToast`
// only, keeping them independent of the Tauri API surface.
(function () {
  const invoke = window.__TAURI__.core.invoke;
  const listen = window.__TAURI__.event.listen;

  window.standup = {
    getState: () => invoke('get_state'),
    setSettings: (patch) => invoke('set_settings', { patch }),
    setStartup: (enabled) => invoke('set_startup', { enabled }),
    minimize: () => invoke('win_minimize'),
    hide: () => invoke('win_hide'),
    onStateUpdate: (cb) => listen('state:update', (e) => cb(e.payload)),
  };

  window.standupToast = {
    done: () => invoke('toast_done'),
    snooze: () => invoke('toast_snooze'),
  };

  // Windows are created hidden; the renderer reveals itself once styled
  // (replaces Electron's ready-to-show).
  window.standupShowWindow = () => {
    window.__TAURI__.window.getCurrentWindow().show().catch(() => {});
  };
})();
