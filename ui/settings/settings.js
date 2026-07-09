'use strict';

const SOUNDS = [
  { id: 'chime', name: 'Chime' },
  { id: 'bell', name: 'Bell' },
  { id: 'marimba', name: 'Marimba' },
  { id: 'softpop', name: 'Soft Pop' },
  { id: 'glass', name: 'Glass' },
  { id: 'droplet', name: 'Droplet' },
];
const DAY_LABELS = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'];
const SEGMENTS = [30, 45, 60, 90, 120];

let state = { settings: null, status: null, startup: null };

const $ = (id) => document.getElementById(id);
const pad = (n) => String(n).padStart(2, '0');

function fmtHM(ms) {
  const d = new Date(ms);
  return pad(d.getHours()) + ':' + pad(d.getMinutes());
}
function fmtCountdown(sec) {
  const h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60;
  if (h > 0) return 'in ' + h + 'h ' + pad(m) + 'm';
  return 'in ' + pad(m) + ':' + pad(s);
}
function daysSummary(days) {
  const on = [];
  days.forEach((v, i) => { if (v) on.push(i); });
  if (on.length === 0) return 'No days';
  if (on.length === 7) return 'Every day';
  const key = on.join(',');
  if (key === '0,1,2,3,4') return 'Weekdays';
  if (key === '5,6') return 'Weekends';
  return on.map(i => DAY_LABELS[i]).join(' ');
}

// ---- theme: follows Windows until the user picks light/dark explicitly ----
const ACCENTS = ['teal', 'indigo', 'violet', 'amber', 'rose'];
const darkMq = window.matchMedia('(prefers-color-scheme: dark)');
function effectiveTheme() {
  const t = state.settings ? state.settings.theme : 'system';
  return t === 'system' ? (darkMq.matches ? 'dark' : 'light') : t;
}
function applyTheme() {
  const theme = effectiveTheme();
  $('root').dataset.theme = theme;
  $('root').dataset.accent = state.settings ? state.settings.accent : 'teal';
  $('btn-theme').textContent = theme === 'dark' ? '☼' : '☾';
}
darkMq.addEventListener('change', applyTheme);
applyTheme();

$('btn-theme').addEventListener('click', () => {
  patch({ theme: effectiveTheme() === 'dark' ? 'light' : 'dark' });
});

const accentMenuEl = $('accent-menu');
ACCENTS.forEach((accent) => {
  const b = document.createElement('button');
  b.className = 'sw-' + accent;
  b.dataset.accent = accent;
  b.setAttribute('aria-label', accent);
  b.addEventListener('click', () => {
    accentMenuEl.hidden = true;
    patch({ accent });
  });
  accentMenuEl.appendChild(b);
});
$('btn-accent').addEventListener('click', (e) => {
  e.stopPropagation();
  accentMenuEl.hidden = !accentMenuEl.hidden;
});
document.addEventListener('click', (e) => {
  if (!accentMenuEl.hidden && !accentMenuEl.contains(e.target) && e.target !== $('btn-accent')) {
    accentMenuEl.hidden = true;
  }
});

// ---- build static control lists ----
const segEl = $('segments');
SEGMENTS.forEach((v) => {
  const b = document.createElement('button');
  b.textContent = String(v);
  b.addEventListener('click', () => patch({ intervalMin: v }));
  segEl.appendChild(b);
});

const daysEl = $('days');
DAY_LABELS.forEach((label, i) => {
  const b = document.createElement('button');
  b.textContent = label;
  b.addEventListener('click', () => {
    const days = state.settings.days.slice();
    days[i] = days[i] ? 0 : 1;
    patch({ days });
  });
  daysEl.appendChild(b);
});

const soundMenuEl = $('sound-menu');
const soundSelectEl = $('sound-select');
SOUNDS.forEach((snd) => {
  const item = document.createElement('button');
  item.dataset.sound = snd.id;

  const dot = document.createElement('span');
  dot.className = 'item-dot';
  const name = document.createElement('span');
  name.textContent = snd.name;

  item.append(dot, name);
  item.addEventListener('click', () => {
    closeSoundMenu();
    patch({ sound: snd.id });
    window.StandupSounds.play(snd.id);
  });
  soundMenuEl.appendChild(item);
});

function closeSoundMenu() {
  soundMenuEl.hidden = true;
  soundSelectEl.classList.remove('open');
}
soundSelectEl.addEventListener('click', (e) => {
  e.stopPropagation();
  soundMenuEl.hidden = !soundMenuEl.hidden;
  soundSelectEl.classList.toggle('open', !soundMenuEl.hidden);
});
document.addEventListener('click', (e) => {
  if (!soundMenuEl.hidden && !soundMenuEl.contains(e.target)) closeSoundMenu();
});
$('sound-preview').addEventListener('click', () => {
  window.StandupSounds.play(state.settings ? state.settings.sound : 'chime');
});

// ---- render ----
function render() {
  const { settings, status, startup } = state;
  if (!settings) return;

  applyTheme();
  accentMenuEl.querySelectorAll('button').forEach((b) => {
    b.classList.toggle('selected', b.dataset.accent === settings.accent);
  });

  $('master-toggle').classList.toggle('on', !!settings.enabled);
  $('controls').classList.toggle('off', !settings.enabled);

  const paused = status.pausedUntil && status.pausedUntil > Date.now();
  const pill = $('status-pill');
  pill.classList.toggle('off', !settings.enabled);
  $('status-word').textContent = settings.enabled ? 'On' : 'Off';

  $('status-time-row').hidden = !settings.enabled;
  $('status-off').hidden = !!settings.enabled;

  if (settings.enabled) {
    if (status.nextFireAt) {
      $('next-time').textContent = fmtHM(status.nextFireAt);
      const rem = Math.max(0, Math.floor((status.nextFireAt - Date.now()) / 1000));
      $('countdown').textContent = fmtCountdown(rem);
    } else {
      $('next-time').textContent = '-';
      $('countdown').textContent = 'no active days';
    }
  }

  let summary = daysSummary(settings.days) + '  ·  ' + settings.start + '-' + settings.end;
  if (settings.enabled && paused) summary += '  ·  Paused until ' + fmtHM(status.pausedUntil);
  $('schedule-summary').textContent = summary;

  segEl.querySelectorAll('button').forEach((b, i) => {
    b.classList.toggle('selected', SEGMENTS[i] === settings.intervalMin);
  });
  daysEl.querySelectorAll('button').forEach((b, i) => {
    b.classList.toggle('selected', !!settings.days[i]);
  });
  const current = SOUNDS.find(s => s.id === settings.sound);
  $('sound-select-name').textContent = current ? current.name : settings.sound;
  soundMenuEl.querySelectorAll('button').forEach((b) => {
    b.classList.toggle('selected', b.dataset.sound === settings.sound);
  });

  if (document.activeElement !== $('start-time')) $('start-time').value = settings.start;
  if (document.activeElement !== $('end-time')) $('end-time').value = settings.end;

  $('startup-row').classList.toggle('unsupported', !startup.supported);
  $('startup-sub').textContent = startup.supported
    ? 'Start with Windows, minimized to the tray'
    : 'Available in the installed app';
  $('startup-toggle').classList.toggle('on', startup.enabled);
}

// ---- wiring ----
async function patch(p) {
  const res = await window.standup.setSettings(p);
  state.settings = res.settings;
  state.status = res.status;
  render();
}

$('master-toggle').addEventListener('click', () => patch({ enabled: !state.settings.enabled }));
$('start-time').addEventListener('change', (e) => { if (e.target.value) patch({ start: e.target.value }); });
$('end-time').addEventListener('change', (e) => { if (e.target.value) patch({ end: e.target.value }); });
$('btn-min').addEventListener('click', () => window.standup.minimize());
$('btn-close').addEventListener('click', () => window.standup.hide());
$('startup-toggle').addEventListener('click', async () => {
  state.startup = await window.standup.setStartup(!state.startup.enabled);
  render();
});

window.standup.onStateUpdate((data) => {
  state = data;
  render();
});

setInterval(render, 1000); // live countdown tick

window.standup.getState().then((data) => {
  state = data;
  render();
  window.standupShowWindow();
});
