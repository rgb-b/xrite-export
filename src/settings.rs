use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use serde_json::Value;

const APP_NAME: &str = "InkDensityTool";

fn default_settings() -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert("illustrator_path".into(), Value::String(String::new()));
    m.insert("ai_template".into(), Value::String(String::new()));
    m.insert("ai_template_extended".into(), Value::String(String::new()));
    m.insert(
        "default_weight_labels".into(),
        serde_json::json!(["120#", "150#", "200#"]),
    );
    m.insert(
        "default_step_labels".into(),
        serde_json::json!(["100", "95", "90", "80", "70", "60", "50", "40", "30", "20", "10", "5", "3", "1"]),
    );
    m.insert("last_session_path".into(), Value::String(String::new()));
    m
}

fn settings_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
        base.join(APP_NAME).join("settings.json")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_NAME)
            .join("settings.json")
    }
}

static CACHE: Lazy<Mutex<Option<HashMap<String, Value>>>> = Lazy::new(|| Mutex::new(None));

pub fn load() -> HashMap<String, Value> {
    let mut cache = CACHE.lock().unwrap();
    if let Some(ref cached) = *cache {
        return HashMap::clone(cached);
    }

    let path = settings_path();
    let stored: HashMap<String, Value> = if path.is_file() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Merge defaults with stored values
    let mut merged = default_settings();
    for (k, v) in stored {
        merged.insert(k, v);
    }

    *cache = Some(merged.clone());
    merged
}

fn save_settings(data: &HashMap<String, Value>) -> Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create settings dir: {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(data)
        .context("Failed to serialize settings")?;
    fs::write(&path, json)
        .with_context(|| format!("Failed to write settings: {}", path.display()))?;
    Ok(())
}

pub fn get(key: &str) -> Option<Value> {
    load().get(key).cloned()
}

pub fn get_str(key: &str) -> String {
    get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

pub fn get_string_vec(key: &str) -> Vec<String> {
    get(key)
        .and_then(|v| v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        }))
        .unwrap_or_default()
}

pub fn set(key: &str, value: Value) {
    let mut data = load();
    data.insert(key.to_string(), value);

    // Update cache
    {
        let mut cache = CACHE.lock().unwrap();
        *cache = Some(data.clone());
    }

    if let Err(e) = save_settings(&data) {
        log::error!("Failed to save settings: {e}");
    }
}

pub fn set_str(key: &str, value: &str) {
    set(key, Value::String(value.to_string()));
}
