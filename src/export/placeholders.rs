use std::collections::HashMap;

use crate::core::models::{JobConfig, ShapeData, WeightData};

const COLOUR_SUFFIXES: &[&str] = &["C", "M", "Y", "K"];

/// Format a float for insertion into templates (strip trailing zeros, blank for 0).
pub fn fmt_value(v: f64) -> String {
    if v == 0.0 {
        return String::new();
    }
    // Equivalent to Python's f"{v:.4g}"
    let s = format!("{:.4}", v);
    // Trim trailing zeros after decimal point
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        // But if we trimmed everything, use the 4g-style format
        if trimmed.is_empty() || trimmed == "-" {
            String::new()
        } else {
            trimmed.to_string()
        }
    } else {
        s
    }
}

/// Build the <<PLACEHOLDER>> → value mapping for one chunk of weights.
pub fn build_placeholders(
    job: &JobConfig,
    shape: &ShapeData,
    chunk: &[&WeightData],
) -> HashMap<String, String> {
    let heading = job.heading();
    let dot_shape = job.dot_shape();

    let mut ph = HashMap::new();
    ph.insert("<<CUSTOMER>>".into(), heading);
    ph.insert("<<STOCK>>".into(), String::new());
    ph.insert("<<CRS>>".into(), dot_shape);
    ph.insert("<<DATE>>".into(), job.date.clone());
    ph.insert(
        "<<SET>>".into(),
        if job.set_number.is_empty() {
            String::new()
        } else {
            format!("Set {}", job.set_number)
        },
    );
    ph.insert(
        "<<JOB>>".into(),
        if job.job_number.is_empty() {
            String::new()
        } else {
            format!("Job {}", job.job_number)
        },
    );
    ph.insert("<<SHAPE>>".into(), shape.name.clone());

    for wn in 1..=3 {
        let weight = chunk.get(wn - 1).copied();
        ph.insert(format!("<<W{wn}_LABEL>>"), weight.map(|w| w.label.clone()).unwrap_or_default());

        // Density row
        for (ci, suffix) in COLOUR_SUFFIXES.iter().enumerate() {
            let val = weight.map(|w| w.density[ci]).unwrap_or(0.0);
            ph.insert(format!("<<W{wn}_D{suffix}>>"), fmt_value(val));
        }

        // Step rows (R01 … R16)
        for ri in 0..16 {
            let rn = ri + 1;
            for (ci, suffix) in COLOUR_SUFFIXES.iter().enumerate() {
                let val = weight.and_then(|w| w.steps.get(ri)).map(|row| row[ci]).unwrap_or(0.0);
                ph.insert(format!("<<W{wn}_R{rn:02}_{suffix}>>"), fmt_value(val));
            }
        }
    }

    ph
}

/// Split weights into groups of up to `size`.
pub fn chunk_weights(weights: &[WeightData], size: usize) -> Vec<Vec<&WeightData>> {
    weights.chunks(size).map(|c| c.iter().collect()).collect()
}
