//! Session save and load.
//!
//! Sessions are stored as pretty-printed JSON. The loader is deliberately
//! lenient — missing fields get sensible defaults and mismatched ink/step
//! counts are padded or trimmed, so old sessions open cleanly in new builds.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::core::models::{default_inks, Ink, InkKind, JobConfig, ShapeData, WeightData};

// ── Save ──────────────────────────────────────────────────────────────────────

/// Write a session to disk atomically (temp → backup → rename).
#[allow(dead_code)] // used in tests; desktop save will call this when re-enabled
pub fn save_session(job: &JobConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(job).context("Failed to serialise job")?;

    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let tmp_path = path.with_file_name(format!("{file_name}.tmp"));

    fs::write(&tmp_path, &json)
        .with_context(|| format!("Failed to write temp file: {}", tmp_path.display()))?;

    // Best-effort backup of the previous version.
    if path.is_file() {
        let bak = path.with_file_name(format!("{file_name}.bak"));
        let _ = fs::copy(path, bak);
    }

    fs::rename(&tmp_path, path)
        .with_context(|| format!("Failed to rename temp to session: {}", path.display()))?;

    Ok(())
}

// ── Load ──────────────────────────────────────────────────────────────────────

/// Load a session from disk. Returns a fully-typed `JobConfig`.
pub fn load_session(path: &Path) -> Result<JobConfig> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read session: {}", path.display()))?;
    let raw: serde_json::Value =
        serde_json::from_str(&data).context("Failed to parse session JSON")?;

    Ok(job_from_value(&raw))
}

// ── Deserialization helpers ───────────────────────────────────────────────────
//
// We deserialise manually rather than relying on serde derive so that we can
// apply pad/trim logic and accept both the old model (fixed [f64;4] per ink)
// and the new model (Vec<f64> per ink).

fn job_from_value(v: &serde_json::Value) -> JobConfig {
    let obj = v.as_object();

    let str = |key: &str| -> String {
        obj.and_then(|o| o.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    let str_vec = |key: &str, defaults: &[&str]| -> Vec<String> {
        obj.and_then(|o| o.get(key))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
            .unwrap_or_else(|| defaults.iter().map(|s| s.to_string()).collect())
    };

    // Resolve inks — support both new format (array of Ink objects) and old
    // format (colour_names string array like ["C","M","Y","K"]).
    let inks: Vec<Ink> = if let Some(arr) = obj
        .and_then(|o| o.get("inks"))
        .and_then(|v| v.as_array())
    {
        arr.iter().filter_map(ink_from_value).collect()
    } else {
        // Fall back to old colour_names field.
        let names = str_vec("colour_names", &["C", "M", "Y", "K"]);
        names.into_iter().map(|n| ink_from_name(&n)).collect()
    };

    let inks = if inks.is_empty() { default_inks() } else { inks };
    let num_inks = inks.len();

    let step_labels = str_vec(
        "step_labels",
        &["100","95","90","80","70","60","50","40","30","20","10","5","3","1"],
    );
    let num_steps = step_labels.len();

    let shapes = obj
        .and_then(|o| o.get("shapes"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(|s| shape_from_value(s, num_inks, num_steps)).collect())
        .unwrap_or_default();

    JobConfig {
        preset_name:  str("preset_name"),
        job_name:     str("job_name"),
        job_number:   str("job_number"),
        customer:     str("customer"),
        plate_tech:   str("plate_tech"),
        press_system: str("press_system"),
        esxr_number:  str("esxr_number"),
        print_type:   str("print_type"),
        date:         str("date"),
        set_number:   str("set_number"),
        inks,
        step_labels,
        shapes,
    }
}

fn shape_from_value(v: &serde_json::Value, num_inks: usize, num_steps: usize) -> ShapeData {
    // Support old format (single "name" field) and new format (dot_type + dot_number).
    let dot_type = v.get("dot_type")
        .and_then(|v| v.as_str())
        .or_else(|| v.get("name").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    let dot_number = v.get("dot_number")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let weights = v.get("weights")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(|w| weight_from_value(w, num_inks, num_steps)).collect())
        .unwrap_or_default();

    ShapeData { dot_type, dot_number, weights }
}

fn weight_from_value(v: &serde_json::Value, num_inks: usize, num_steps: usize) -> WeightData {
    // Support old "label" field and new "lpi" field.
    let lpi = v.get("lpi")
        .or_else(|| v.get("label"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let density = pad_f64_vec(
        v.get("density")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
            .unwrap_or_default(),
        num_inks,
    );

    let steps: Vec<Vec<f64>> = v.get("steps")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|row| {
                    pad_f64_vec(
                        row.as_array()
                            .map(|r| r.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
                            .unwrap_or_default(),
                        num_inks,
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    // Ensure step row count matches expected count.
    let mut w = WeightData { lpi, density, steps };
    w.resize_steps(num_steps, num_inks);
    w
}

/// Pad or trim a `Vec<f64>` to exactly `len` elements.
fn pad_f64_vec(mut v: Vec<f64>, len: usize) -> Vec<f64> {
    v.resize(len, 0.0);
    v
}

/// Parse an Ink object from JSON (new format).
fn ink_from_value(v: &serde_json::Value) -> Option<Ink> {
    let name = v.get("name")?.as_str()?.to_string();
    let kind_str = v.get("kind")?.as_str()?;
    let kind = match kind_str {
        "cyan"    => InkKind::Cyan,
        "magenta" => InkKind::Magenta,
        "yellow"  => InkKind::Yellow,
        "black"   => InkKind::Black,
        "white"   => InkKind::White,
        "spot"    => InkKind::Spot,
        _ => return None,
    };
    Some(Ink { kind, name })
}

/// Map old colour_names strings to Ink values (old format fallback).
fn ink_from_name(name: &str) -> Ink {
    match name {
        "C" => Ink::cyan(),
        "M" => Ink::magenta(),
        "Y" => Ink::yellow(),
        "K" => Ink::black(),
        "W" => Ink::white(),
        _   => Ink::spot(name),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_job() -> JobConfig {
        JobConfig {
            customer:    "Acme".into(),
            plate_tech:  "CRS".into(),
            press_system:"XPS".into(),
            print_type:  "RP".into(),
            shapes: vec![ShapeData {
                dot_type:   "CRS".into(),
                dot_number: "501".into(),
                weights: vec![WeightData::new("150#", 4, 14)],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn round_trip() {
        let job = make_job();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        save_session(&job, tmp.path()).unwrap();
        let loaded = load_session(tmp.path()).unwrap();

        assert_eq!(loaded.customer,    "Acme");
        assert_eq!(loaded.plate_tech,  "CRS");
        assert_eq!(loaded.print_type,  "RP");
        assert_eq!(loaded.shapes[0].dot_type,   "CRS");
        assert_eq!(loaded.shapes[0].dot_number, "501");
        assert_eq!(loaded.shapes[0].weights[0].lpi, "150#");
        assert_eq!(loaded.shapes[0].weights[0].steps.len(), 14);
        assert_eq!(loaded.shapes[0].weights[0].density.len(), 4);
    }

    #[test]
    fn old_format_colour_names() {
        // Simulate an old-format session JSON with colour_names instead of inks.
        let json = serde_json::json!({
            "customer": "OldCo",
            "colour_names": ["C","M","Y","K"],
            "shapes": []
        });
        let job = job_from_value(&json);
        assert_eq!(job.inks.len(), 4);
        assert_eq!(job.inks[0].name, "C");
    }

    #[test]
    fn step_resize_on_load() {
        // Session saved with 14 steps, loaded into a 16-step context.
        let job = make_job();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        save_session(&job, tmp.path()).unwrap();

        // Manually patch the JSON to have step_labels with 16 entries.
        let mut raw: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(tmp.path()).unwrap()).unwrap();
        raw["step_labels"] = serde_json::json!([
            "100","95","90","80","70","60","50","40","30","20","10","5","3","1","0.8","0.4"
        ]);
        std::fs::write(tmp.path(), serde_json::to_string_pretty(&raw).unwrap()).unwrap();

        let loaded = load_session(tmp.path()).unwrap();
        assert_eq!(loaded.shapes[0].weights[0].steps.len(), 16);
    }
}
