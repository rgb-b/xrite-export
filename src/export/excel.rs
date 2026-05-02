//! Excel export — generates .xlsx from scratch (no template required).
//!
//! One sheet per shape-LPI combination (e.g. "CRS 501 - 150#").
//!
//! Sheet layout (row numbers):
//!   1   Job heading                      │  last col: date
//!   2   Shape name — LPI label
//!   3   Column headers: Step │ inks… │ Avg │ Dev
//!   4   Max Density row
//!   5+  Step rows (100% first, then descending)
//!
//! Avg = average of CMYK-kind inks only.
//! Dev = Avg − interpolated target (blank if step has no target).
//! Values are written as formatted strings; no live Excel formulas.

use std::path::Path;

use anyhow::{Context, Result};

use crate::core::models::{JobConfig, ShapeData, WeightData};
use crate::core::targets::interpolate_target;

// ── Column / address helpers ──────────────────────────────────────────────────

fn col_letter(col: u32) -> String {
    let mut result = String::new();
    let mut c = col;
    while c > 0 {
        c -= 1;
        result.insert(0, char::from(b'A' + (c % 26) as u8));
        c /= 26;
    }
    result
}

fn addr(col: u32, row: u32) -> String {
    format!("{}{}", col_letter(col), row)
}

// ── Value formatters ──────────────────────────────────────────────────────────

fn fmt_density(v: f64) -> String {
    if v == 0.0 { String::new() } else { format!("{:.2}", v) }
}

fn fmt_step(v: f64) -> String {
    if v == 0.0 { String::new() } else { format!("{:.1}", v) }
}

fn fmt_dev(v: f64) -> String {
    if v.abs() < 0.05 {
        "0".to_string()
    } else {
        format!("{:+.1}", v)
    }
}

// ── Export entry point ────────────────────────────────────────────────────────

pub fn export_excel(job: &JobConfig, output_path: &Path) -> Result<()> {
    let mut book = umya_spreadsheet::new_file();

    // Collect all (shape, weight) pairs to generate one sheet each
    let sheet_data: Vec<(&ShapeData, &WeightData)> = job
        .shapes
        .iter()
        .flat_map(|shape| shape.weights.iter().map(move |weight| (shape, weight)))
        .collect();

    if sheet_data.is_empty() {
        // No data — write an empty workbook
    } else {
        // Clone the initial empty sheet before writing (used as base for extras)
        let base_ws = book.get_sheet_collection()[0].clone();

        // Rename the first sheet
        let first_name = sheet_name(sheet_data[0].0, sheet_data[0].1);
        book.get_sheet_collection_mut()[0].set_name(&first_name);

        // Create additional sheets
        for (shape, weight) in &sheet_data[1..] {
            let mut ws = base_ws.clone();
            ws.set_name(sheet_name(shape, weight));
            book.get_sheet_collection_mut().push(ws);
        }

        // Write data to each sheet
        for (idx, (shape, weight)) in sheet_data.iter().enumerate() {
            let ws = book
                .get_sheet_mut(&idx)
                .ok_or_else(|| anyhow::anyhow!("Sheet {} not found", idx))?;
            write_weight_sheet(ws, job, shape, weight);
        }
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output dir: {}", parent.display()))?;
    }

    umya_spreadsheet::writer::xlsx::write(&book, output_path)
        .map_err(|e| anyhow::anyhow!("Failed to write Excel: {:?}", e))?;

    Ok(())
}

fn sheet_name(shape: &ShapeData, weight: &WeightData) -> String {
    // Excel sheet names have a 31-char limit; truncate if needed
    let name = if weight.lpi.is_empty() {
        shape.display_name()
    } else {
        format!("{} - {}", shape.display_name(), weight.lpi)
    };
    // Sanitise: Excel forbids [ ] : * ? / \  in sheet names
    let sanitised: String = name
        .chars()
        .map(|c| match c {
            '[' | ']' | ':' | '*' | '?' | '/' | '\\' => '_',
            _ => c,
        })
        .take(31)
        .collect();
    if sanitised.is_empty() { "Sheet".to_string() } else { sanitised }
}

// ── Sheet writer ──────────────────────────────────────────────────────────────

fn write_weight_sheet(
    ws: &mut umya_spreadsheet::Worksheet,
    job: &JobConfig,
    shape: &ShapeData,
    weight: &WeightData,
) {
    let num_inks = job.num_inks();
    let avg_col = num_inks as u32 + 2; // column after all ink columns
    let dev_col = avg_col + 1;
    let deviation_indices = job.deviation_ink_indices();

    // ── Row 1: heading + date ────────────────────────────────────────────────
    let heading = job.heading();
    if !heading.is_empty() {
        ws.get_cell_mut(addr(1, 1)).set_value(heading);
    }
    if !job.date.is_empty() {
        ws.get_cell_mut(addr(avg_col, 1)).set_value(job.date.clone());
    }

    // ── Row 2: shape — lpi ──────────────────────────────────────────────────
    let shape_label = if weight.lpi.is_empty() {
        shape.display_name()
    } else {
        format!("{} \u{2014} {}", shape.display_name(), weight.lpi)
    };
    ws.get_cell_mut(addr(1, 2)).set_value(shape_label);

    // ── Row 3: column headers ────────────────────────────────────────────────
    ws.get_cell_mut(addr(1, 3)).set_value("Step");
    for (ci, ink) in job.inks.iter().enumerate() {
        ws.get_cell_mut(addr(ci as u32 + 2, 3)).set_value(ink.name.clone());
    }
    ws.get_cell_mut(addr(avg_col, 3)).set_value("Avg");
    ws.get_cell_mut(addr(dev_col, 3)).set_value("Dev");

    // ── Row 4: max density ───────────────────────────────────────────────────
    ws.get_cell_mut(addr(1, 4)).set_value("Max Density");
    for (ci, &v) in weight.density.iter().enumerate() {
        if v != 0.0 {
            ws.get_cell_mut(addr(ci as u32 + 2, 4)).set_value(fmt_density(v));
        }
    }

    // ── Rows 5+: step rows ───────────────────────────────────────────────────
    for (si, label) in job.step_labels.iter().enumerate() {
        let row = 5u32 + si as u32;
        ws.get_cell_mut(addr(1, row)).set_value(label.clone());

        // 100% row is always 100 for all inks; other rows use stored data
        let row_values: Vec<f64> = if label == "100" {
            vec![100.0; num_inks]
        } else {
            let stored = weight.steps.get(si).cloned().unwrap_or_default();
            let mut v = stored;
            v.resize(num_inks, 0.0);
            v
        };

        for (ci, &v) in row_values.iter().enumerate() {
            if v != 0.0 {
                ws.get_cell_mut(addr(ci as u32 + 2, row)).set_value(fmt_step(v));
            }
        }

        // Avg (CMYK inks only)
        if !deviation_indices.is_empty() {
            let sum: f64 = deviation_indices
                .iter()
                .map(|&i| row_values.get(i).copied().unwrap_or(0.0))
                .sum();
            let avg = sum / deviation_indices.len() as f64;
            ws.get_cell_mut(addr(avg_col, row)).set_value(fmt_step(avg));

            // Dev = avg − interpolated target for this step
            if let Ok(step_f) = label.parse::<f64>() {
                if let Some(target) = interpolate_target(step_f) {
                    let dev = avg - target;
                    ws.get_cell_mut(addr(dev_col, row)).set_value(fmt_dev(dev));
                }
            }
        }
    }
}
