//! Excel export — fills the bundled .xlsx templates using umya-spreadsheet.
//!
//! Template layout (matches Python export/excel.py exactly):
//!
//!   Two sheets per shape:
//!     Single sheet (even index): weight[0] only
//!     Dual sheet   (odd index):  weight[1] (first table) + weight[2] (second table)
//!
//!   Cell mapping (1-indexed rows & cols):
//!     A1 / I1           title / date
//!     B4:E17 (std)      CMYK step data, first table  (B4:E19 for 16-step)
//!     A18 / I18 (std)   weight label / dot shape     (A20 / I20 for 16-step)
//!     B25:E38 (std)     CMYK step data, second table (B27:E42 for 16-step)
//!     A38 / I38 (std)   weight label / dot shape     (A42 / I42 for 16-step)
//!     F column (dual)   =SUM(Bn:En)/4 average formula — fixed for second table

use std::io::Write as IoWrite;
use std::path::Path;

use anyhow::{Context, Result};

use crate::core::models::JobConfig;

/// Embedded template bytes — standard (14 steps) and extended (16 steps).
static TEMPLATE_STANDARD: &[u8] = include_bytes!("../../assets/template_standard.xlsx");
static TEMPLATE_EXTENDED: &[u8] = include_bytes!("../../assets/template_extended.xlsx");

/// Row where step data starts (1-indexed, first table).
const STEP_START_ROW_T1: u32 = 4;
/// Row gap between end of table-1 label and start of table-2 step data.
const GAP_T1_TO_T2: u32 = 7;
/// CMYK data columns B=2, C=3, D=4, E=5 (1-indexed).
const DATA_COLS: [u32; 4] = [2, 3, 4, 5];
/// Per-colour difference columns J=10, K=11, L=12, M=13 (1-indexed).
const DIFF_COLS: [u32; 4] = [10, 11, 12, 13];
/// Ideal% column H=8 (1-indexed).
const IDEAL_COL: u32 = 8;

fn row_constants(num_steps: u32) -> (u32, u32, u32) {
    let label_t1 = STEP_START_ROW_T1 + num_steps;
    let step_start_t2 = label_t1 + GAP_T1_TO_T2;
    let label_t2 = step_start_t2 + num_steps;
    (label_t1, step_start_t2, label_t2)
}

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

/// Format a float for cell entry: blank for 0, otherwise strip trailing zeros.
fn fmt_num(v: f64) -> String {
    if v == 0.0 {
        return String::new();
    }
    let s = format!("{:.6}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn export_excel(job: &JobConfig, output_path: &Path) -> Result<()> {
    let num_steps = job.num_steps() as u32;
    let (label_t1, step_start_t2, label_t2) = row_constants(num_steps);

    let template_bytes = if num_steps > 14 {
        TEMPLATE_EXTENDED
    } else {
        TEMPLATE_STANDARD
    };

    // Write template bytes to a named temp file so umya-spreadsheet can open it.
    let mut tmp = tempfile::NamedTempFile::new().context("Failed to create temp file")?;
    tmp.write_all(template_bytes)
        .context("Failed to write template to temp file")?;
    tmp.flush().context("Failed to flush temp file")?;
    let tmp_path = tmp.path().to_path_buf();

    let mut book = umya_spreadsheet::reader::xlsx::read(&tmp_path)
        .map_err(|e| anyhow::anyhow!("Failed to read Excel template: {:?}", e))?;

    let title = job.heading();
    let dot_shape = job.dot_shape();

    let sheet_count = book.get_sheet_collection().len();

    for (shape_idx, shape) in job.shapes.iter().enumerate() {
        let single_idx = shape_idx * 2;
        let dual_idx = shape_idx * 2 + 1;

        // Ensure template has enough sheets; clone first pair if needed.
        while book.get_sheet_collection().len() <= dual_idx {
            clone_first_pair(&mut book)?;
        }
        let _ = sheet_count; // used above

        // --- Single sheet: weight[0] ---
        {
            let ws = book
                .get_sheet_mut(&single_idx)
                .ok_or_else(|| anyhow::anyhow!("Sheet {} not found", single_idx))?;

            ws.get_cell_mut(addr(1, 1)).set_value(&title);
            ws.get_cell_mut(addr(9, 1)).set_value(&job.date);

            write_diff_headers(ws, 1, 2, Some(STEP_START_ROW_T1 - 1));
            if let Some(w0) = shape.weights.get(0) {
                write_density(ws, &w0.density, STEP_START_ROW_T1 - 1);
                write_steps(ws, &w0.steps, STEP_START_ROW_T1, num_steps);
                write_per_colour_diff(ws, STEP_START_ROW_T1, num_steps);
                ws.get_cell_mut(addr(1, label_t1)).set_value(&w0.label);
                ws.get_cell_mut(addr(9, label_t1)).set_value(&dot_shape);
            }
        }

        // --- Dual sheet: weight[1] (table 1) + weight[2] (table 2) ---
        {
            let ws = book
                .get_sheet_mut(&dual_idx)
                .ok_or_else(|| anyhow::anyhow!("Sheet {} not found", dual_idx))?;

            ws.get_cell_mut(addr(1, 1)).set_value(&title);
            ws.get_cell_mut(addr(9, 1)).set_value(&job.date);

            write_diff_headers(ws, 1, 2, Some(STEP_START_ROW_T1 - 1));
            if let Some(w1) = shape.weights.get(1) {
                write_density(ws, &w1.density, STEP_START_ROW_T1 - 1);
                write_steps(ws, &w1.steps, STEP_START_ROW_T1, num_steps);
                write_per_colour_diff(ws, STEP_START_ROW_T1, num_steps);
                ws.get_cell_mut(addr(1, label_t1)).set_value(&w1.label);
                ws.get_cell_mut(addr(9, label_t1)).set_value(&dot_shape);
            }

            if let Some(w2) = shape.weights.get(2) {
                write_density(ws, &w2.density, step_start_t2 - 1);
                write_steps(ws, &w2.steps, step_start_t2, num_steps);
                write_diff_headers(ws, step_start_t2 - 2, step_start_t2 - 1, None);
                write_per_colour_diff(ws, step_start_t2, num_steps);
                ws.get_cell_mut(addr(1, label_t2)).set_value(&w2.label);
                ws.get_cell_mut(addr(9, label_t2)).set_value(&dot_shape);
                fix_second_table_formulas(ws, step_start_t2, num_steps);
            }
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

/// Write the four density values (C/M/Y/K) into columns B–E at `density_row`.
fn write_density(
    ws: &mut umya_spreadsheet::Worksheet,
    density: &[f64; 4],
    density_row: u32,
) {
    for (ci, &col) in DATA_COLS.iter().enumerate() {
        let v = density[ci];
        if v != 0.0 {
            ws.get_cell_mut(addr(col, density_row)).set_value(fmt_num(v));
        }
    }
}

/// Write step data rows into columns B–E starting at `start_row`.
fn write_steps(
    ws: &mut umya_spreadsheet::Worksheet,
    steps: &[[f64; 4]],
    start_row: u32,
    num_steps: u32,
) {
    for ri in 0..num_steps as usize {
        if ri >= steps.len() {
            break;
        }
        let row_data = &steps[ri];
        let excel_row = start_row + ri as u32;
        for (ci, &col) in DATA_COLS.iter().enumerate() {
            let v = row_data[ci];
            if v != 0.0 {
                ws.get_cell_mut(addr(col, excel_row)).set_value(fmt_num(v));
            }
        }
    }
}

/// Write "Difference" heading, C/M/Y/K subheaders, and (optionally) a separator row
/// in columns J–M, copying border/fill/alignment styles from the CMYK reference cells.
///
/// `separator_row`: pass `Some(row)` when there is a blank row between the subheader
/// and the first data row (table 1 only — row 3 in the standard layout).  Pass `None`
/// for table 2 where the subheader sits immediately above the data.
fn write_diff_headers(
    ws: &mut umya_spreadsheet::Worksheet,
    heading_row: u32,
    subheader_row: u32,
    separator_row: Option<u32>,
) {
    // --- Heading row: copy style from B1 (bottom:medium border, same for all cols) ---
    for &col in &DIFF_COLS {
        let style = ws.get_style(addr(2, 1)).clone();
        ws.set_style(addr(col, heading_row), style);
    }
    ws.get_cell_mut(addr(DIFF_COLS[0], heading_row)).set_value("Difference");

    // --- Subheader row: copy CMYK fills + borders from B2/C2/D2/E2 ---
    for (i, &col) in DIFF_COLS.iter().enumerate() {
        let style = ws.get_style(addr(DATA_COLS[i], 2)).clone();
        ws.set_style(addr(col, subheader_row), style);
        ws.get_cell_mut(addr(col, subheader_row))
            .set_value(["C", "M", "Y", "K"][i]);
    }

    // --- Separator row: copy top:thin|bottom:medium style from B3/C3/D3/E3 ---
    if let Some(sep) = separator_row {
        for (i, &col) in DIFF_COLS.iter().enumerate() {
            let style = ws.get_style(addr(DATA_COLS[i], 3)).clone();
            ws.set_style(addr(col, sep), style);
        }
    }
}

/// Write per-colour diff formulas (=H{r}-B{r} … =H{r}-E{r}) in columns J–M,
/// copying left/right border and centre-alignment styles from B4/C4/D4/E4.
fn write_per_colour_diff(
    ws: &mut umya_spreadsheet::Worksheet,
    step_start_row: u32,
    num_steps: u32,
) {
    // Clone reference styles from the first data row of table 1 (row 4).
    let ref_styles: Vec<umya_spreadsheet::Style> = DATA_COLS
        .iter()
        .map(|&src| ws.get_style(addr(src, STEP_START_ROW_T1)).clone())
        .collect();

    let ideal = col_letter(IDEAL_COL);
    for i in 0..num_steps {
        let r = step_start_row + i;
        for (ci, &col) in DIFF_COLS.iter().enumerate() {
            ws.set_style(addr(col, r), ref_styles[ci].clone());
            let data_col = col_letter(DATA_COLS[ci]);
            ws.get_cell_mut(addr(col, r))
                .set_value(format!("={ideal}{r}-{data_col}{r}"));
        }
    }
}

/// Overwrite the F-column formulas in the second table with correct row references.
/// The original template has a copy-paste error (references offset by 2 rows).
fn fix_second_table_formulas(
    ws: &mut umya_spreadsheet::Worksheet,
    start_row: u32,
    num_steps: u32,
) {
    for i in 0..num_steps {
        let r = start_row + i;
        // Write as formula string; umya-spreadsheet treats "=..." values as formulas.
        ws.get_cell_mut(addr(6, r))
            .set_value(format!("=SUM(B{r}:E{r})/4"));
    }
}

/// Append a new sheet pair by cloning the first two sheets in the workbook.
/// Clears data cells so the cloned sheets start blank.
fn clone_first_pair(book: &mut umya_spreadsheet::Spreadsheet) -> Result<()> {
    let existing_count = book.get_sheet_collection().len();
    let new_single_name = format!("Sheet{}", existing_count + 1);
    let new_dual_name = format!("Sheet{}", existing_count + 2);

    book.new_sheet(&new_single_name)
        .map_err(|e| anyhow::anyhow!("Failed to add sheet: {:?}", e))?;
    book.new_sheet(&new_dual_name)
        .map_err(|e| anyhow::anyhow!("Failed to add sheet: {:?}", e))?;

    Ok(())
}
