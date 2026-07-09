use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub const INTERVALS: [u32; 5] = [30, 45, 60, 90, 120];
pub const SOUNDS: [&str; 6] = ["chime", "bell", "marimba", "softpop", "glass", "droplet"];
pub const THEMES: [&str; 3] = ["system", "light", "dark"];
pub const ACCENTS: [&str; 5] = ["teal", "indigo", "violet", "amber", "rose"];

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub enabled: bool,
    pub interval_min: u32,
    pub start: String,
    pub end: String,
    pub days: Vec<u8>, // Monday-based
    pub sound: String,
    pub theme: String,
    pub accent: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            enabled: true,
            interval_min: 60,
            start: "09:00".into(),
            end: "18:00".into(),
            days: vec![1, 1, 1, 1, 1, 0, 0],
            sound: "chime".into(),
            theme: "system".into(),
            accent: "teal".into(),
        }
    }
}

fn valid_hm(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 2 || c.is_ascii_digit())
        && s[..2].parse::<u32>().map(|h| h < 24).unwrap_or(false)
        && s[3..].parse::<u32>().map(|m| m < 60).unwrap_or(false)
}

// Field-wise validation: a corrupt or partial file falls back per field,
// never crashes the app.
pub fn sanitize(raw: &Value) -> Settings {
    let mut s = Settings::default();
    if let Some(b) = raw.get("enabled").and_then(Value::as_bool) {
        s.enabled = b;
    }
    if let Some(i) = raw.get("intervalMin").and_then(Value::as_u64) {
        if INTERVALS.contains(&(i as u32)) {
            s.interval_min = i as u32;
        }
    }
    for (key, field) in [("start", &mut s.start), ("end", &mut s.end)] {
        if let Some(v) = raw.get(key).and_then(Value::as_str) {
            if valid_hm(v) {
                *field = v.into();
            }
        }
    }
    if let Some(days) = raw.get("days").and_then(Value::as_array) {
        if days.len() == 7 {
            s.days = days
                .iter()
                .map(|d| {
                    let truthy = d.as_bool().unwrap_or(false)
                        || d.as_f64().map(|n| n != 0.0).unwrap_or(false);
                    truthy as u8
                })
                .collect();
        }
    }
    for (key, field, allowed) in [
        ("sound", &mut s.sound, &SOUNDS[..]),
        ("theme", &mut s.theme, &THEMES[..]),
        ("accent", &mut s.accent, &ACCENTS[..]),
    ] {
        if let Some(v) = raw.get(key).and_then(Value::as_str) {
            if allowed.contains(&v) {
                *field = v.into();
            }
        }
    }
    s
}

pub fn apply_patch(current: &Settings, patch: &Value) -> Settings {
    let mut v = serde_json::to_value(current).unwrap_or(Value::Null);
    if let (Some(obj), Some(p)) = (v.as_object_mut(), patch.as_object()) {
        for (k, val) in p {
            obj.insert(k.clone(), val.clone());
        }
    }
    sanitize(&v)
}

// Same file as v1.0.x, so upgrading users keep their settings.
fn file_path() -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").unwrap_or_default())
        .join("Standup")
        .join("settings.json")
}

pub fn load() -> Settings {
    let raw = fs::read_to_string(file_path())
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .unwrap_or(Value::Null);
    sanitize(&raw)
}

pub fn save(s: &Settings) {
    let file = file_path();
    if let Some(dir) = file.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let tmp = file.with_extension("json.tmp");
    if let Ok(text) = serde_json::to_string_pretty(s) {
        if fs::write(&tmp, text).is_ok() {
            let _ = fs::rename(&tmp, &file);
        }
    }
}
