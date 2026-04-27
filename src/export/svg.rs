//! SVG export — generates a single SVG file matching the Illustrator template layout.

use crate::core::models::JobConfig;

// ── Layout constants (match build_ai_template.jsx exactly) ────────────────────
const PAGE_W: f64 = 842.0;
const PAGE_H: f64 = 595.0;
const MARGIN: f64 = 20.0;
const HEADER_H: f64 = 44.0;
const LABEL_ROW_H: f64 = 18.0;
const COL_HDR_ROW_H: f64 = 14.0;
const ROW_H: f64 = 14.0;
const DATA_COL_W: f64 = 42.0;
const WEIGHT_GAP: f64 = 6.0;
const FONT_SIZE: f64 = 7.0;
const HEADER_FONT_SIZE: f64 = 9.0;
const LABEL_FONT_SIZE: f64 = 8.0;

// Derived constants
const CONTENT_W: f64 = PAGE_W - 2.0 * MARGIN;       // 802.0
const BLOCK_W: f64 = 4.0 * DATA_COL_W;              // 168.0
const WEIGHT_AREA_W: f64 = 3.0 * BLOCK_W + 2.0 * WEIGHT_GAP; // 516.0
const STEP_COL_W: f64 = CONTENT_W - WEIGHT_AREA_W;  // 286.0
const X0: f64 = MARGIN;                              // 20.0

fn w_block_x(i: usize) -> f64 {
    X0 + STEP_COL_W + i as f64 * (BLOCK_W + WEIGHT_GAP)
}

const Y_HEADER: f64 = MARGIN;         // 20.0
const Y_LABELS: f64 = Y_HEADER + HEADER_H;  // 64.0
const Y_COL_HDR: f64 = Y_LABELS + LABEL_ROW_H;      // 82.0
const Y_DENSITY: f64 = Y_COL_HDR + COL_HDR_ROW_H;  // 96.0

fn y_row(i: usize) -> f64 {
    Y_DENSITY + ROW_H + i as f64 * ROW_H
}

/// Escape XML special characters.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Format a data value: empty string when 0.0, else 2 decimal places.
fn fmt_val(v: f64) -> String {
    if v == 0.0 {
        String::new()
    } else {
        format!("{v:.2}")
    }
}

/// `<rect>` with no fill, black stroke.
fn cell_rect(x: f64, y: f64, w: f64, h: f64) -> String {
    format!(
        r##"<rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" fill="none" stroke="#000" stroke-width="0.25"/>"##
    )
}

/// Cell with both rect and text.
fn cell(x: f64, y: f64, w: f64, h: f64, text: &str, fs: f64, bold: bool) -> String {
    let mut s = cell_rect(x, y, w, h);
    if !text.is_empty() {
        let weight = if bold { "bold" } else { "normal" };
        let tx = x + 2.0;
        let ty = y + h / 2.0;
        s.push_str(&format!(
            r#"<text x="{tx:.2}" y="{ty:.2}" font-size="{fs}" font-family="Helvetica, Arial, sans-serif" font-weight="{weight}" dominant-baseline="middle">{}</text>"#,
            esc(text)
        ));
    }
    s
}

/// Render one page (chunk of ≤3 weights for a shape) offset by `page_top`.
fn render_page(
    job: &JobConfig,
    shape_name: &str,
    weights: &[&crate::core::models::WeightData],
    page_top: f64,
) -> String {
    let num_steps = job.step_labels.len();
    let mut out = String::new();

    let pt = page_top;

    // White background
    out.push_str(&format!(
        r#"<rect x="0" y="{pt:.2}" width="{PAGE_W}" height="{PAGE_H}" fill="white"/>"#
    ));

    // ── Header row 1 ──────────────────────────────────────────────────────────
    let hx = X0;
    let hy = pt + Y_HEADER;

    // Customer | print_type | DATE | set_number | job_number
    let header1_fields = [
        job.customer.as_str(),
        job.print_type.as_str(),
        &job.date,
        &job.set_number,
        &job.job_number,
    ];
    let header1_labels = ["CUSTOMER", "PRINT TYPE", "DATE", "SET No.", "JOB No."];
    let col_widths = [200.0f64, 120.0, 120.0, 180.0, 182.0];
    let mut cx = hx;
    for i in 0..5 {
        let w = col_widths[i];
        let label = format!("{}: {}", header1_labels[i], header1_fields[i]);
        out.push_str(&cell(cx, hy, w, HEADER_H / 2.0, &label, HEADER_FONT_SIZE, false));
        cx += w;
    }

    // Header row 2: shape name spanning full width
    let hy2 = hy + HEADER_H / 2.0;
    out.push_str(&cell(hx, hy2, CONTENT_W, HEADER_H / 2.0, shape_name, HEADER_FONT_SIZE, true));

    // ── Weight blocks ─────────────────────────────────────────────────────────
    let colours = &job.colour_names;

    for (wi, weight) in weights.iter().enumerate() {
        let wx = w_block_x(wi);

        // Label row
        let ly = pt + Y_LABELS;
        out.push_str(&cell(wx, ly, BLOCK_W, LABEL_ROW_H, &weight.label, LABEL_FONT_SIZE, true));

        // Colour header row
        let chy = pt + Y_COL_HDR;
        for (ci, colour) in colours.iter().enumerate().take(4) {
            out.push_str(&cell(wx + ci as f64 * DATA_COL_W, chy, DATA_COL_W, COL_HDR_ROW_H, colour, FONT_SIZE, true));
        }

        // Density row
        let dy = pt + Y_DENSITY;
        for ci in 0..4 {
            let v = if ci < weight.density.len() { weight.density[ci] } else { 0.0 };
            out.push_str(&cell(wx + ci as f64 * DATA_COL_W, dy, DATA_COL_W, ROW_H, &fmt_val(v), FONT_SIZE, false));
        }

        // Step rows
        for (ri, row) in weight.steps.iter().enumerate().take(num_steps) {
            let ry = pt + y_row(ri);
            for ci in 0..4 {
                let v = if ci < row.len() { row[ci] } else { 0.0 };
                out.push_str(&cell(wx + ci as f64 * DATA_COL_W, ry, DATA_COL_W, ROW_H, &fmt_val(v), FONT_SIZE, false));
            }
        }
    }

    // ── Step gutter ───────────────────────────────────────────────────────────
    // "D" header above density row
    out.push_str(&cell(X0, pt + Y_COL_HDR, STEP_COL_W, COL_HDR_ROW_H, "D", FONT_SIZE, true));

    // Density label row placeholder (matches weight label height)
    out.push_str(&cell(X0, pt + Y_LABELS, STEP_COL_W, LABEL_ROW_H, "", FONT_SIZE, false));

    // Density value row label
    out.push_str(&cell(X0, pt + Y_DENSITY, STEP_COL_W, ROW_H, "Density", FONT_SIZE, false));

    // Step labels
    for (ri, label) in job.step_labels.iter().enumerate().take(num_steps) {
        let ry = pt + y_row(ri);
        out.push_str(&cell(X0, ry, STEP_COL_W, ROW_H, label, FONT_SIZE, false));
    }

    // ── Outer border ─────────────────────────────────────────────────────────
    let table_top = pt + Y_LABELS;
    let table_h = LABEL_ROW_H + COL_HDR_ROW_H + ROW_H + num_steps as f64 * ROW_H;
    out.push_str(&format!(
        r##"<rect x="{X0:.2}" y="{table_top:.2}" width="{CONTENT_W:.2}" height="{table_h:.2}" fill="none" stroke="#000" stroke-width="0.5"/>"##
    ));

    out
}

/// Generate a single SVG for the full job.
pub fn export_svg(job: &JobConfig) -> String {
    let mut pages: Vec<String> = Vec::new();
    let mut page_index = 0usize;

    for shape in &job.shapes {
        for chunk in shape.weights.chunks(3) {
            let weight_refs: Vec<&crate::core::models::WeightData> = chunk.iter().collect();
            let pt = page_index as f64 * PAGE_H;
            let page_svg = render_page(job, &shape.name, &weight_refs, pt);
            pages.push(format!(
                r#"<g id="page{page_index}">{page_svg}</g>"#
            ));
            page_index += 1;
        }
    }

    // If no shapes, render a blank page
    if pages.is_empty() {
        pages.push(format!(
            r#"<g id="page0"><rect x="0" y="0" width="{PAGE_W}" height="{PAGE_H}" fill="white"/></g>"#
        ));
        page_index = 1;
    }

    let total_h = page_index as f64 * PAGE_H;
    let body = pages.join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{PAGE_W}" height="{total_h:.2}" viewBox="0 0 {PAGE_W} {total_h:.2}"
     xmlns="http://www.w3.org/2000/svg"
     xmlns:xlink="http://www.w3.org/1999/xlink">
<style>text {{ font-family: Helvetica, Arial, sans-serif; }}</style>
{body}
</svg>"#
    )
}
