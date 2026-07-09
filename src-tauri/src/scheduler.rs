use chrono::{DateTime, Datelike, Duration, Local, Timelike};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::{store::Settings, toast, AppState};

const SNOOZE_MIN: i64 = 5;
const PAUSE_MIN: i64 = 60;
const MISSED_GRACE_MS: i64 = 3 * 60 * 1000;

#[derive(Default)]
pub struct Sched {
    pub paused_until: Option<i64>, // epoch ms, in-memory only
    pub snooze_at: Option<i64>,    // epoch ms, in-memory only
    pub next_fire_at: Option<i64>, // epoch ms
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    pub paused_until: Option<i64>,
    pub next_fire_at: Option<i64>,
}

pub fn parse_hm(s: &str) -> u32 {
    let mut it = s.split(':').map(|p| p.parse::<u32>().unwrap_or(0));
    let h = it.next().unwrap_or(0);
    let m = it.next().unwrap_or(0);
    h * 60 + m
}

fn now_ms() -> i64 {
    Local::now().timestamp_millis()
}

// Interval buckets are aligned to midnight so reminders land on round times;
// Monday-based day index; scans up to 8 days ahead.
pub fn compute_next(
    now: DateTime<Local>,
    i_min: u32,
    start_min: u32,
    end_min: u32,
    days: &[u8],
) -> Option<DateTime<Local>> {
    let i = i_min.max(1);
    let first_b = start_min.div_ceil(i) * i;
    let last_b = (end_min / i) * i;
    if first_b > last_b {
        return None;
    }
    for d in 0..8i64 {
        let date = (now + Duration::days(d)).date_naive();
        let idx = date.weekday().num_days_from_monday() as usize;
        if days.get(idx).copied().unwrap_or(0) == 0 {
            continue;
        }
        let b = if d == 0 {
            // +1: the result must be strictly after `now`. Truncated seconds
            // land exactly on the slot that just fired, which would re-arm it
            // and fire a second time one tick later.
            let now_sec = now.num_seconds_from_midnight() + 1;
            let mut b = now_sec.div_ceil(i * 60) * i;
            if b < first_b {
                b = first_b;
            }
            if b > last_b {
                continue;
            }
            b
        } else {
            first_b
        };
        let naive = date.and_hms_opt(0, 0, 0)? + Duration::minutes(b as i64);
        if let Some(t) = naive.and_local_timezone(Local).earliest() {
            return Some(t);
        }
    }
    None
}

fn effective(state: &AppState) -> Settings {
    let mut s = state.settings.lock().unwrap().clone();
    if let Some(d) = state.debug_interval {
        s.interval_min = d;
    }
    s
}

fn compute_next_fire(state: &AppState, from_ms: i64) -> Option<i64> {
    let s = effective(state);
    if !s.enabled {
        return None;
    }
    let (snooze, paused) = {
        let sched = state.sched.lock().unwrap();
        (sched.snooze_at, sched.paused_until)
    };
    if let Some(sn) = snooze {
        return Some(sn); // snooze fires unconditionally
    }
    let mut base = from_ms;
    if let Some(p) = paused {
        if p > base {
            base = p;
        }
    }
    let dt = DateTime::from_timestamp_millis(base)?.with_timezone(&Local);
    compute_next(
        dt,
        s.interval_min,
        parse_hm(&s.start),
        parse_hm(&s.end),
        &s.days,
    )
    .map(|t| t.timestamp_millis())
}

fn still_in_window(state: &AppState, at_ms: i64) -> bool {
    let s = effective(state);
    let Some(d) = DateTime::from_timestamp_millis(at_ms) else {
        return false;
    };
    let d = d.with_timezone(&Local);
    let now_min = d.hour() * 60 + d.minute();
    now_min >= parse_hm(&s.start)
        && now_min <= parse_hm(&s.end)
        && s.days
            .get(d.date_naive().weekday().num_days_from_monday() as usize)
            .copied()
            .unwrap_or(0)
            != 0
}

pub fn status(state: &AppState) -> Status {
    let enabled = state.settings.lock().unwrap().enabled;
    let sched = state.sched.lock().unwrap();
    Status {
        enabled,
        paused_until: sched.paused_until,
        next_fire_at: sched.next_fire_at,
    }
}

pub fn rearm(app: &AppHandle) {
    let state = app.state::<AppState>();
    let now = now_ms();
    {
        let mut sched = state.sched.lock().unwrap();
        if matches!(sched.paused_until, Some(p) if p <= now) {
            sched.paused_until = None;
        }
    }
    let next = compute_next_fire(&state, now);
    state.sched.lock().unwrap().next_fire_at = next;
    crate::emit_state(app);
}

pub fn snooze(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.sched.lock().unwrap().snooze_at = Some(now_ms() + SNOOZE_MIN * 60 * 1000);
    rearm(app);
}

pub fn toggle_pause(app: &AppHandle) {
    let state = app.state::<AppState>();
    {
        let mut sched = state.sched.lock().unwrap();
        if matches!(sched.paused_until, Some(p) if p > now_ms()) {
            sched.paused_until = None;
        } else {
            sched.paused_until = Some(now_ms() + PAUSE_MIN * 60 * 1000);
            sched.snooze_at = None;
        }
    }
    rearm(app);
}

// Runs every second from a background thread: a passed slot fires (within
// the grace window), and a stale computation resyncs - which also covers
// waking from sleep, clock changes, and settings edits from any path.
pub fn tick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let now = now_ms();
    let stored = state.sched.lock().unwrap().next_fire_at;
    match stored {
        Some(next) if now >= next => {
            let late = now - next;
            let was_snooze = {
                let mut sched = state.sched.lock().unwrap();
                let w = sched.snooze_at.is_some();
                sched.snooze_at = None;
                w
            };
            let fire = late <= MISSED_GRACE_MS && (was_snooze || still_in_window(&state, now));
            rearm(app);
            if fire {
                toast::show(app);
            }
        }
        _ => {
            let pause_expired =
                matches!(state.sched.lock().unwrap().paused_until, Some(p) if p <= now);
            let expected = compute_next_fire(&state, now);
            if expected != stored || pause_expired {
                rearm(app);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const ALL_DAYS: [u8; 7] = [1; 7];

    fn at(h: u32, m: u32, s: u32) -> DateTime<Local> {
        // Wed 2026-07-08, an active day in every test schedule
        Local.with_ymd_and_hms(2026, 7, 8, h, m, s).unwrap()
    }

    fn next_hm(now: DateTime<Local>) -> (u32, u32) {
        let t = compute_next(now, 60, 9 * 60, 18 * 60, &ALL_DAYS).unwrap();
        (t.hour(), t.minute())
    }

    // Regression: at the exact second a slot fires, re-arming must return the
    // NEXT slot - returning the same one made the toast fire twice (v1.1.0).
    #[test]
    fn rearm_at_fire_second_skips_to_next_slot() {
        assert_eq!(next_hm(at(10, 0, 0)), (11, 0));
    }

    #[test]
    fn rearm_just_after_fire_skips_to_next_slot() {
        assert_eq!(next_hm(at(10, 0, 1)), (11, 0));
        assert_eq!(next_hm(at(10, 30, 0)), (11, 0));
    }

    #[test]
    fn before_active_hours_arms_first_slot() {
        assert_eq!(next_hm(at(7, 15, 0)), (9, 0));
    }

    #[test]
    fn just_before_slot_still_arms_that_slot() {
        assert_eq!(next_hm(at(9, 59, 59)), (10, 0));
    }

    #[test]
    fn after_last_slot_rolls_to_next_day() {
        let t = compute_next(at(18, 30, 0), 60, 9 * 60, 18 * 60, &ALL_DAYS).unwrap();
        assert_eq!((t.day(), t.hour(), t.minute()), (9, 9, 0));
    }

    #[test]
    fn skips_inactive_days() {
        // Only Sunday active; Wed 2026-07-08 -> Sun 2026-07-12
        let days = [0, 0, 0, 0, 0, 0, 1];
        let t = compute_next(at(10, 0, 0), 60, 9 * 60, 18 * 60, &days).unwrap();
        assert_eq!((t.day(), t.hour()), (12, 9));
    }

    #[test]
    fn no_active_days_returns_none() {
        assert!(compute_next(at(10, 0, 0), 60, 9 * 60, 18 * 60, &[0; 7]).is_none());
    }

    #[test]
    fn interval_45_aligns_to_midnight_buckets() {
        // 45-min buckets: ...09:00, 09:45, 10:30... (aligned to 00:00)
        let t = compute_next(at(9, 50, 0), 45, 9 * 60, 18 * 60, &ALL_DAYS).unwrap();
        assert_eq!((t.hour(), t.minute()), (10, 30));
    }
}
