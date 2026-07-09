'use strict';

const AUTO_DISMISS_MS = 2 * 60 * 1000;

// Config comes from an init script injected by the Rust side.
const params = new URLSearchParams(window.__TOAST_QS__ || location.search);
const theme = params.get('theme') || 'system';
const darkMq = window.matchMedia('(prefers-color-scheme: dark)');
function applyTheme() {
  const root = document.getElementById('root');
  root.dataset.theme = theme === 'system' ? (darkMq.matches ? 'dark' : 'light') : theme;
  root.dataset.accent = params.get('accent') || 'teal';
}
darkMq.addEventListener('change', applyTheme);
applyTheme();

const sound = params.get('sound') || 'chime';
window.standupShowWindow();
window.StandupSounds.play(sound);

function leave(then) {
  const toast = document.getElementById('toast');
  toast.classList.add('leaving');
  setTimeout(then, 150);
}

document.getElementById('btn-done').addEventListener('click', () => leave(() => window.standupToast.done()));
document.getElementById('btn-close').addEventListener('click', () => leave(() => window.standupToast.done()));
document.getElementById('btn-snooze').addEventListener('click', () => leave(() => window.standupToast.snooze()));

setTimeout(() => leave(() => window.standupToast.done()), AUTO_DISMISS_MS);
