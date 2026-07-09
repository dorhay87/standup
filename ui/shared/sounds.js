// Sound synthesis ported verbatim from the approved mockup (Standup.html).
'use strict';
(function () {
  let _ac = null;

  function ensureAudio() {
    if (!_ac) {
      const AC = window.AudioContext || window.webkitAudioContext;
      _ac = new AC();
    }
    if (_ac.state === 'suspended') _ac.resume();
    return _ac;
  }

  function tone(o) {
    const ac = ensureAudio();
    const t0 = o.when != null ? o.when : ac.currentTime;
    const osc = ac.createOscillator();
    osc.type = o.type || 'sine';
    osc.frequency.setValueAtTime(o.freq, t0);
    if (o.glideTo) osc.frequency.exponentialRampToValueAtTime(o.glideTo, t0 + (o.dur || 0.3));
    if (o.detune) osc.detune.value = o.detune;
    const g = ac.createGain();
    const peak = o.gain != null ? o.gain : 0.2;
    const a = o.attack != null ? o.attack : 0.006;
    const d = o.dur != null ? o.dur : 0.3;
    g.gain.setValueAtTime(0.0001, t0);
    g.gain.exponentialRampToValueAtTime(peak, t0 + a);
    g.gain.exponentialRampToValueAtTime(0.0001, t0 + a + d);
    if (o.lowpass) {
      const f = ac.createBiquadFilter();
      f.type = 'lowpass'; f.frequency.value = o.lowpass;
      osc.connect(f); f.connect(g);
    } else {
      osc.connect(g);
    }
    g.connect(ac.destination);
    osc.start(t0);
    osc.stop(t0 + a + d + 0.06);
  }

  function play(id) {
    const ac = ensureAudio();
    const n = ac.currentTime + 0.01;
    switch (id) {
      case 'chime':
        tone({ freq: 1046.5, gain: 0.18, dur: 0.5, when: n });
        tone({ freq: 1318.5, gain: 0.16, dur: 0.55, when: n + 0.09 });
        tone({ freq: 1567.98, gain: 0.11, dur: 0.65, when: n + 0.18 });
        break;
      case 'bell':
        tone({ freq: 660, gain: 0.2, dur: 1.1, when: n });
        tone({ freq: 660 * 2.76, gain: 0.06, dur: 0.9, when: n });
        tone({ freq: 660 * 5.4, gain: 0.03, dur: 0.6, when: n });
        break;
      case 'marimba':
        tone({ freq: 523.25, type: 'triangle', gain: 0.22, dur: 0.28, when: n, lowpass: 2600 });
        tone({ freq: 1046.5, gain: 0.07, dur: 0.22, when: n });
        tone({ freq: 659.25, type: 'triangle', gain: 0.18, dur: 0.3, when: n + 0.13, lowpass: 2600 });
        break;
      case 'softpop':
        tone({ freq: 523, gain: 0.24, dur: 0.12, attack: 0.004, when: n, lowpass: 1700 });
        tone({ freq: 784, gain: 0.1, dur: 0.14, when: n + 0.02 });
        break;
      case 'glass':
        tone({ freq: 1567.98, gain: 0.13, dur: 0.6, when: n, detune: 5 });
        tone({ freq: 2093, gain: 0.09, dur: 0.7, when: n + 0.04, detune: -5 });
        tone({ freq: 3135.96, gain: 0.05, dur: 0.5, when: n + 0.02 });
        break;
      case 'droplet':
        tone({ freq: 900, glideTo: 480, gain: 0.24, dur: 0.18, when: n, lowpass: 2200 });
        break;
      default:
        tone({ freq: 880, gain: 0.2, dur: 0.3, when: n });
    }
  }

  window.StandupSounds = { play };
})();
