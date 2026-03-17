use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::core::models::{JobConfig, ShapeData, WeightData};

/// Ensure a Vec<f64> has exactly `length` elements, padding with 0.0 or trimming.
fn pad_or_trim(v: &[f64], length: usize) -> Vec<f64> {
    let mut out = v.to_vec();
    out.resize(length, 0.0);
    out.truncate(length);
    out
}

/// Ensure a [f64; 4] from a variable-length source.
fn to_array4(v: &[f64]) -> [f64; 4] {
    let padded = pad_or_trim(v, 4);
    [padded[0], padded[1], padded[2], padded[3]]
}

/// Raw JSON shape for forward-compatible deserialization.
/// We deserialize into raw Value first, then manually construct our typed structs
/// to handle missing/extra fields gracefully (matching Python's _pad_or_trim approach).
pub fn save_session(job: &JobConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(job)
        .context("Failed to serialize job")?;
    fs::write(path, json)
        .with_context(|| format!("Failed to write session: {}", path.display()))?;
    Ok(())
}

pub fn load_session(path: &Path) -> Result<JobConfig> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read session: {}", path.display()))?;
    let raw: serde_json::Value = serde_json::from_str(&data)
        .context("Failed to parse session JSON")?;

    Ok(job_from_value(&raw))
}

fn job_from_value(v: &serde_json::Value) -> JobConfig {
    let obj = v.as_object();

    let get_str = |key: &str, default: &str| -> String {
        obj.and_then(|o| o.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string()
    };

    let get_string_vec = |key: &str, defaults: &[&str]| -> Vec<String> {
        obj.and_then(|o| o.get(key))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|v| v.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_else(|| defaults.iter().map(|s| s.to_string()).collect())
    };

    let shapes = obj
        .and_then(|o| o.get("shapes"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(shape_from_value).collect())
        .unwrap_or_default();

    JobConfig {
        customer: get_str("customer", ""),
        print_type: get_str("print_type", "CRS"),
        stock_desc: get_str("stock_desc", ""),
        finish: get_str("finish", "RP"),
        dot_shape_type: get_str("dot_shape_type", "CRS"),
        dot_shape_number: get_str("dot_shape_number", ""),
        date: get_str("date", ""),
        set_number: get_str("set_number", ""),
        job_number: get_str("job_number", ""),
        weight_labels: get_string_vec("weight_labels", &["120#", "150#", "200#"]),
        step_labels: get_string_vec(
            "step_labels",
            &["100", "95", "90", "80", "70", "60", "50", "40", "30", "20", "10", "5", "3", "1"],
        ),
        colour_names: get_string_vec("colour_names", &["C", "M", "Y", "K"]),
        shapes,
    }
}

fn shape_from_value(v: &serde_json::Value) -> ShapeData {
    let name = v.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let weights = v
        .get("weights")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(weight_from_value).collect())
        .unwrap_or_default();
    ShapeData { name, weights }
}

fn weight_from_value(v: &serde_json::Value) -> WeightData {
    let label = v.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let raw_density: Vec<f64> = v
        .get("density")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
        .unwrap_or_else(|| vec![0.0; 4]);
    let density = to_array4(&raw_density);

    let steps: Vec<[f64; 4]> = v
        .get("steps")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|row| {
                    let vals: Vec<f64> = row
                        .as_array()
                        .map(|r| r.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
                        .unwrap_or_else(|| vec![0.0; 4]);
                    to_array4(&vals)
                })
                .collect()
        })
        .unwrap_or_else(|| vec![[0.0; 4]]);

    WeightData {
        label,
        density,
        steps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_session() {
        let job = JobConfig {
            customer: "Test".into(),
            shapes: vec![ShapeData {
                name: "Shape 1".into(),
                weights: vec![WeightData::new("120#", 14)],
            }],
            ..Default::default()
        };

        let tmp = tempfile::NamedTempFile::new().unwrap();
        save_session(&job, tmp.path()).unwrap();
        let loaded = load_session(tmp.path()).unwrap();

        assert_eq!(loaded.customer, "Test");
        assert_eq!(loaded.shapes.len(), 1);
        assert_eq!(loaded.shapes[0].weights[0].label, "120#");
        assert_eq!(loaded.shapes[0].weights[0].steps.len(), 14);
    }

    #[test]
    fn pad_or_trim_works() {
        assert_eq!(pad_or_trim(&[1.0, 2.0], 4), vec![1.0, 2.0, 0.0, 0.0]);
        assert_eq!(pad_or_trim(&[1.0, 2.0, 3.0, 4.0, 5.0], 4), vec![1.0, 2.0, 3.0, 4.0]);
    }
}
