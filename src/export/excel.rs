//! Excel export — fills user-provided .xlsx templates using umya-spreadsheet.
//!
//! Template layout (matches user-configured cell map; defaults shown):
//!
//!   Two sheets per shape:
//!     Single sheet (even index): weight[0] only
//!     Dual sheet   (odd index):  weight[1] (first table) + weight[2] (second table)
//!
//!   Cell mapping (1-indexed rows & cols, all configurable via Settings → Cell Mapping):
//!     A1 / I1           title / date
//!     B3:E3             CMYK density values
//!     B4:E17 (std)      CMYK step data, first table  (B4:E19 for 16-step)
//!     A18 / I18 (std)   weight label / dot shape     (A20 / I20 for 16-step)
//!     — Dual sheet second section —
//!     A19 / I19 (std)   title / date for second section (A21 / I21 for 16-step)
//!     B22:E35 (std)     CMYK step data, second table (B24:E39 for 16-step)
//!     A36 / I36 (std)   weight label / dot shape     (A40 / I40 for 16-step)

use std::path::Path;

use anyhow::{Context, Result};

use crate::core::models::JobConfig;
use crate::settings;

// ── Cell map ─────────────────────────────────────────────────────────────────

/// Configurable mapping of data fields to Excel columns/rows.
/// All columns are 1-indexed (A=1, B=2, …).
pub struct ExcelCellMap {
    pub title_col:          u32, // default 1  (A)
    pub date_col:           u32, // default 9  (I)
    pub step_start_row:     u32, // default 4
    pub data_col_c:         u32, // default 2  (B)
    pub data_col_m:         u32, // default 3  (C)
    pub data_col_y:         u32, // default 4  (D)
    pub data_col_k:         u32, // default 5  (E)
    pub label_col:          u32, // default 1  (A)
    pub dot_shape_col:      u32, // default 9  (I)
    pub gap_t1_to_t2:       u32, // default 4
    pub density_row_offset: u32, // default 1  (density row = step_start_row - density_row_offset)
    pub title_t2_row_offset: u32, // default 3 (title2 row = step_start_t2 - title_t2_row_offset)
}

impl ExcelCellMap {
    pub fn from_settings() -> Self {
        let col = |key: &str, default: u32| -> u32 {
            let s = settings::get_str(key);
            col_letter_to_num(s.trim()).unwrap_or(default)
        };
        let row = |key: &str, default: u32| -> u32 {
            let s = settings::get_str(key);
            s.trim().parse::<u32>().unwrap_or(default)
        };

        Self {
            title_col:           col("xcm_title_col",           1),
            date_col:            col("xcm_date_col",             9),
            step_start_row:      row("xcm_step_start_row",       4),
            data_col_c:          col("xcm_data_col_c",           2),
            data_col_m:          col("xcm_data_col_m",           3),
            data_col_y:          col("xcm_data_col_y",           4),
            data_col_k:          col("xcm_data_col_k",           5),
            label_col:           col("xcm_label_col",            1),
            dot_shape_col:       col("xcm_dot_shape_col",        9),
            gap_t1_to_t2:        row("xcm_gap_t1_to_t2",        4),
            density_row_offset:  row("xcm_density_row_offset",  1),
            title_t2_row_offset: row("xcm_title_t2_row_offset", 3),
        }
    }

    fn data_cols(&self) -> [u32; 4] {
        [self.data_col_c, self.data_col_m, self.data_col_y, self.data_col_k]
    }
}

/// Parse an Excel column letter(s) into a 1-indexed number (case-insensitive).
/// Returns `None` if the string contains non-alpha characters or is empty.
fn col_letter_to_num(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    let mut result: u32 = 0;
    for c in s.chars() {
        let upper = c.to_ascii_uppercase();
        if !upper.is_ascii_alphabetic() {
            return None;
        }
        result = result * 26 + (upper as u32 - b'A' as u32 + 1);
    }
    Some(result)
}

// ── Row helpers ───────────────────────────────────────────────────────────────

/// Returns `(label_t1, step_start_t2, label_t2)` given the number of steps.
fn row_constants(num_steps: u32, cm: &ExcelCellMap) -> (u32, u32, u32) {
    let label_t1 = cm.step_start_row + num_steps;
    let step_start_t2 = label_t1 + cm.gap_t1_to_t2;
    let label_t2 = step_start_t2 + num_steps;
    (label_t1, step_start_t2, label_t2)
}

// ── Cell address helpers ──────────────────────────────────────────────────────

/// Convert a 1-indexed column number to its Excel letter(s).
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

/// Build an A1-notation cell address from 1-indexed column and row.
fn addr(col: u32, row: u32) -> String {
    format!("{}{}", col_letter(col), row)
}

/// Format a density value: blank for 0, otherwise 2 decimal places.
fn fmt_density(v: f64) -> String {
    if v == 0.0 { String::new() } else { format!("{:.2}", v) }
}

/// Format a step value: blank for 0, otherwise 1 decimal place.
fn fmt_step(v: f64) -> String {
    if v == 0.0 { String::new() } else { format!("{:.1}", v) }
}

// ── Export entry point ────────────────────────────────────────────────────────

pub fn export_excel(job: &JobConfig, output_path: &Path) -> Result<()> {
    let cm = ExcelCellMap::from_settings();
    let num_steps = job.num_steps() as u32;
    let (label_t1, step_start_t2, label_t2) = row_constants(num_steps, &cm);
    let data_cols = cm.data_cols();

    let tmpl_path_str = settings::get_str("excel_template");
    if tmpl_path_str.is_empty() {
        anyhow::bail!(
            "Excel template path not set. Please configure it under Settings → Templates."
        );
    }
    let tmpl_path = std::path::Path::new(&tmpl_path_str);
    if !tmpl_path.exists() {
        anyhow::bail!("Excel template not found: {}", tmpl_path.display());
    }

    let mut book = umya_spreadsheet::reader::xlsx::read(tmpl_path)
        .map_err(|e| anyhow::anyhow!("Failed to read Excel template: {:?}", e))?;

    // The template has 4 sheets: 0=single-std, 1=dual-std, 2=single-ext, 3=dual-ext.
    // For extended step counts (>14) use the ext pair (offset 2), otherwise standard (offset 0).
    let sheet_offset: usize = if num_steps > 14 { 2 } else { 0 };

    let title = job.heading();
    let dot_shape = job.dot_shape();

    // Pre-clone all needed sheets from the clean template pair BEFORE writing
    // any data, so clones always come from pristine template sheets.
    let total_needed = sheet_offset + job.shapes.len() * 2;
    while book.get_sheet_collection().len() < total_needed {
        let n = book.get_sheet_collection().len();
        let mut s = book
            .get_sheet_collection()
            .get(sheet_offset)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Template sheet at index {} not found", sheet_offset))?;
        s.set_name(format!("Sheet{}", n + 1));
        let mut d = book
            .get_sheet_collection()
            .get(sheet_offset + 1)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Template sheet at index {} not found", sheet_offset + 1))?;
        d.set_name(format!("Sheet{}", n + 2));
        book.get_sheet_collection_mut().push(s);
        book.get_sheet_collection_mut().push(d);
    }

    for (shape_idx, shape) in job.shapes.iter().enumerate() {
        let single_idx = sheet_offset + shape_idx * 2;
        let dual_idx = sheet_offset + shape_idx * 2 + 1;

        // --- Single sheet: weight[0] ---
        {
            let ws = book
                .get_sheet_mut(&single_idx)
                .ok_or_else(|| anyhow::anyhow!("Sheet {} not found", single_idx))?;

            ws.get_cell_mut(addr(cm.title_col, 1)).set_value(&title);
            ws.get_cell_mut(addr(cm.date_col, 1)).set_value(&job.date);
            ws.get_cell_mut(addr(cm.title_col, 2)).set_value(&shape.name);

            if let Some(w0) = shape.weights.get(0) {
                write_density(ws, &w0.density, cm.step_start_row - cm.density_row_offset, &data_cols);
                write_hundred_row(ws, cm.step_start_row, &data_cols);
                write_steps(ws, &w0.steps[1..], cm.step_start_row + 1, &data_cols);
                ws.get_cell_mut(addr(cm.label_col, label_t1)).set_value(&w0.label);
                ws.get_cell_mut(addr(cm.dot_shape_col, label_t1)).set_value(&dot_shape);
            }
        }

        // --- Dual sheet: weight[1] (table 1) + weight[2] (table 2) ---
        {
            let ws = book
                .get_sheet_mut(&dual_idx)
                .ok_or_else(|| anyhow::anyhow!("Sheet {} not found", dual_idx))?;

            ws.get_cell_mut(addr(cm.title_col, 1)).set_value(&title);
            ws.get_cell_mut(addr(cm.date_col, 1)).set_value(&job.date);
            ws.get_cell_mut(addr(cm.title_col, 2)).set_value(&shape.name);

            if let Some(w1) = shape.weights.get(1) {
                write_density(ws, &w1.density, cm.step_start_row - cm.density_row_offset, &data_cols);
                write_hundred_row(ws, cm.step_start_row, &data_cols);
                write_steps(ws, &w1.steps[1..], cm.step_start_row + 1, &data_cols);
                ws.get_cell_mut(addr(cm.label_col, label_t1)).set_value(&w1.label);
                ws.get_cell_mut(addr(cm.dot_shape_col, label_t1)).set_value(&dot_shape);
            }

            if let Some(w2) = shape.weights.get(2) {
                // Title and date for the second section.
                let title_t2_row = step_start_t2 - cm.title_t2_row_offset;
                ws.get_cell_mut(addr(cm.title_col, title_t2_row)).set_value(&title);
                ws.get_cell_mut(addr(cm.date_col, title_t2_row)).set_value(&job.date);

                write_density(ws, &w2.density, step_start_t2 - cm.density_row_offset, &data_cols);
                write_hundred_row(ws, step_start_t2, &data_cols);
                write_steps(ws, &w2.steps[1..], step_start_t2 + 1, &data_cols);
                ws.get_cell_mut(addr(cm.label_col, label_t2)).set_value(&w2.label);
                ws.get_cell_mut(addr(cm.dot_shape_col, label_t2)).set_value(&dot_shape);
                // F-column average formulas are preserved from the cloned template — do not overwrite.
            }
        }
    }

    // Remove the unused standard-template sheets (indices 0..sheet_offset) when the
    // extended template pair was used, so the output doesn't contain blank sheets.
    if sheet_offset > 0 {
        book.get_sheet_collection_mut().drain(0..sheet_offset);
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output dir: {}", parent.display()))?;
    }

    umya_spreadsheet::writer::xlsx::write(&book, output_path)
        .map_err(|e| anyhow::anyhow!("Failed to write Excel: {:?}", e))?;

    Ok(())
}

// ── Cell writers ──────────────────────────────────────────────────────────────

/// Write the four density values (C/M/Y/K) into the configured data columns at `density_row`.
fn write_density(
    ws: &mut umya_spreadsheet::Worksheet,
    density: &[f64; 4],
    density_row: u32,
    data_cols: &[u32; 4],
) {
    for (ci, &col) in data_cols.iter().enumerate() {
        let v = density[ci];
        if v != 0.0 {
            ws.get_cell_mut(addr(col, density_row)).set_value(fmt_density(v));
        }
    }
}

/// Write 100.0 into every data column at `row` (the locked 100% step row).
fn write_hundred_row(ws: &mut umya_spreadsheet::Worksheet, row: u32, data_cols: &[u32; 4]) {
    for &col in data_cols.iter() {
        ws.get_cell_mut(addr(col, row)).set_value("100");
    }
}

/// Write step data rows into the configured data columns starting at `start_row`.
fn write_steps(
    ws: &mut umya_spreadsheet::Worksheet,
    steps: &[[f64; 4]],
    start_row: u32,
    data_cols: &[u32; 4],
) {
    for (ri, row_data) in steps.iter().enumerate() {
        let excel_row = start_row + ri as u32;
        for (ci, &col) in data_cols.iter().enumerate() {
            let v = row_data[ci];
            if v != 0.0 {
                ws.get_cell_mut(addr(col, excel_row)).set_value(fmt_step(v));
            }
        }
    }
}

/// Overwrite the F-column average formulas in the second table with correct row references.
fn fix_second_table_formulas(
    ws: &mut umya_spreadsheet::Worksheet,
    start_row: u32,
    num_steps: u32,
) {
    for i in 0..num_steps {
        let r = start_row + i;
        ws.get_cell_mut(addr(6, r))
            .set_formula(format!("SUM(B{r}:E{r})/4"));
    }
}

// ── Sheet cloning ─────────────────────────────────────────────────────────────

/// Append a cloned sheet pair by copying the previous two sheets.
fn clone_first_pair(book: &mut umya_spreadsheet::Spreadsheet) -> Result<()> {
    let existing_count = book.get_sheet_collection().len();
    if existing_count < 2 {
        anyhow::bail!("Template has fewer than 2 sheets; cannot clone pair");
    }

    let mut cloned_single = book
        .get_sheet_collection()
        .get(existing_count - 2)
        .ok_or_else(|| anyhow::anyhow!("Source single sheet not found"))?
        .clone();
    cloned_single.set_name(format!("Sheet{}", existing_count + 1));

    let mut cloned_dual = book
        .get_sheet_collection()
        .get(existing_count - 1)
        .ok_or_else(|| anyhow::anyhow!("Source dual sheet not found"))?
        .clone();
    cloned_dual.set_name(format!("Sheet{}", existing_count + 2));

    book.get_sheet_collection_mut().push(cloned_single);
    book.get_sheet_collection_mut().push(cloned_dual);

    Ok(())
}

