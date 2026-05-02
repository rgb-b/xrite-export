//! SVG export — generates a multi-page SVG matching the Illustrator template layout.
//!
//! Pages are stacked vertically (each PAGE_H units apart).
//! Up to 3 weight blocks fit per page; columns scale with ink count.

use crate::core::models::{JobConfig, WeightData};

// ── Fixed layout constants ────────────────────────────────────────────────────
const PAGE_W: f64 = 842.0;
const PAGE_H: f64 = 595.0;
const MARGIN: f64 = 20.0;
const HEADER_H: f64 = 44.0;
const LABEL_ROW_H: f64 = 18.0;
const COL_HDR_ROW_H: f64 = 14.0;
const ROW_H: f64 = 14.0;
const DATA_COL_W: f64 = 40.0;
const WEIGHT_GAP: f64 = 6.0;
const FONT_SIZE: f64 = 7.0;
const HEADER_FONT_SIZE: f64 = 9.0;
const LABEL_FONT_SIZE: f64 = 8.0;

const CONTENT_W: f64 = PAGE_W - 2.0 * MARGIN;
const X0: f64 = MARGIN;

const Y_HEADER: f64 = MARGIN;
const Y_LABELS: f64 = Y_HEADER + HEADER_H;
const Y_COL_HDR: f64 = Y_LABELS + LABEL_ROW_H;
const Y_DENSITY: f64 = Y_COL_HDR + COL_HDR_ROW_H;

fn y_row(i: usize) -> f64 {
    Y_DENSITY + ROW_H + i as f64 * ROW_H
}

// ── Layout (depends on ink count) ─────────────────────────────────────────────

struct Layout {
    num_inks: usize,
    block_w: f64,
    step_col_w: f64,
}

impl Layout {
    fn new(num_inks: usize) -> Self {
        let num_inks = num_inks.max(1);
        let block_w = num_inks as f64 * DATA_COL_W;
        let weight_area_w = 3.0 * block_w + 2.0 * WEIGHT_GAP;
        let step_col_w = (CONTENT_W - weight_area_w).max(60.0);
        Self { num_inks, block_w, step_col_w }
    }

    fn w_block_x(&self, i: usize) -> f64 {
        X0 + self.step_col_w + i as f64 * (self.block_w + WEIGHT_GAP)
    }
}

// ── SVG primitives ────────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn fmt_val(v: f64) -> String {
    if v == 0.0 { String::new() } else { format!("{v:.2}") }
}

fn cell_rect(x: f64, y: f64, w: f64, h: f64) -> String {
    format!(
        r##"<rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" fill="none" stroke="#000" stroke-width="0.25"/>"##
    )
}

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

// ── Page renderer ─────────────────────────────────────────────────────────────

fn render_page(
    job: &JobConfig,
    shape_name: &str,
    weights: &[&WeightData],
    page_top: f64,
) -> String {
    let ink_names: Vec<&str> = job.inks.iter().map(|i| i.name.as_str()).collect();
    let layout = Layout::new(ink_names.len());
    let num_steps = job.step_labels.len();
    let mut out = String::new();
    let pt = page_top;

    // White background
    out.push_str(&format!(
        r#"<rect x="0" y="{pt:.2}" width="{PAGE_W}" height="{PAGE_H}" fill="white"/>"#
    ));

    // ── Header ────────────────────────────────────────────────────────────────
    let hx = X0;
    let hy = pt + Y_HEADER;

    let header_fields = [
        ("CUSTOMER", job.customer.as_str()),
        ("PRINT TYPE", job.print_type.as_str()),
        ("DATE", job.date.as_str()),
        ("SET No.", job.set_number.as_str()),
        ("JOB No.", job.job_number.as_str()),
    ];
    let col_widths = [200.0f64, 120.0, 120.0, 180.0, 182.0];
    let mut cx = hx;
    for (i, (label, value)) in header_fields.iter().enumerate() {
        let w = col_widths[i];
        let text = format!("{label}: {value}");
        out.push_str(&cell(cx, hy, w, HEADER_H / 2.0, &text, HEADER_FONT_SIZE, false));
        cx += w;
    }
    // Shape name row
    let hy2 = hy + HEADER_H / 2.0;
    out.push_str(&cell(hx, hy2, CONTENT_W, HEADER_H / 2.0, shape_name, HEADER_FONT_SIZE, true));

    // ── Weight blocks ─────────────────────────────────────────────────────────
    for (wi, weight) in weights.iter().enumerate() {
        let wx = layout.w_block_x(wi);

        // LPI label row
        let ly = pt + Y_LABELS;
        out.push_str(&cell(wx, ly, layout.block_w, LABEL_ROW_H, &weight.lpi, LABEL_FONT_SIZE, true));

        // Ink header row
        let chy = pt + Y_COL_HDR;
        for (ci, &ink_name) in ink_names.iter().enumerate() {
            out.push_str(&cell(
                wx + ci as f64 * DATA_COL_W,
                chy,
                DATA_COL_W,
                COL_HDR_ROW_H,
                ink_name,
                FONT_SIZE,
                true,
            ));
        }

        // Density row
        let dy = pt + Y_DENSITY;
        for ci in 0..layout.num_inks {
            let v = weight.density.get(ci).copied().unwrap_or(0.0);
            out.push_str(&cell(
                wx + ci as f64 * DATA_COL_W,
                dy,
                DATA_COL_W,
                ROW_H,
                &fmt_val(v),
                FONT_SIZE,
                false,
            ));
        }

        // Step rows
        for (ri, row) in weight.steps.iter().enumerate().take(num_steps) {
            let ry = pt + y_row(ri);
            for ci in 0..layout.num_inks {
                let v = row.get(ci).copied().unwrap_or(0.0);
                out.push_str(&cell(
                    wx + ci as f64 * DATA_COL_W,
                    ry,
                    DATA_COL_W,
                    ROW_H,
                    &fmt_val(v),
                    FONT_SIZE,
                    false,
                ));
            }
        }
    }

    // ── Step gutter (left column) ─────────────────────────────────────────────
    // "D" header above density row
    out.push_str(&cell(X0, pt + Y_COL_HDR, layout.step_col_w, COL_HDR_ROW_H, "D", FONT_SIZE, true));
    // LPI label placeholder
    out.push_str(&cell(X0, pt + Y_LABELS, layout.step_col_w, LABEL_ROW_H, "", FONT_SIZE, false));
    // "Density" label
    out.push_str(&cell(X0, pt + Y_DENSITY, layout.step_col_w, ROW_H, "Density", FONT_SIZE, false));
    // Step labels
    for (ri, label) in job.step_labels.iter().enumerate().take(num_steps) {
        let ry = pt + y_row(ri);
        out.push_str(&cell(X0, ry, layout.step_col_w, ROW_H, label, FONT_SIZE, false));
    }

    // ── Outer border ─────────────────────────────────────────────────────────
    let table_top = pt + Y_LABELS;
    let table_h = LABEL_ROW_H + COL_HDR_ROW_H + ROW_H + num_steps as f64 * ROW_H;
    out.push_str(&format!(
        r##"<rect x="{X0:.2}" y="{table_top:.2}" width="{CONTENT_W:.2}" height="{table_h:.2}" fill="none" stroke="#000" stroke-width="0.5"/>"##
    ));

    out
}

// ── Public export function ────────────────────────────────────────────────────

pub fn export_svg(job: &JobConfig) -> String {
    let mut pages: Vec<String> = Vec::new();
    let mut page_index = 0usize;

    for shape in &job.shapes {
        let shape_name = shape.display_name();
        for chunk in shape.weights.chunks(3) {
            let weight_refs: Vec<&WeightData> = chunk.iter().collect();
            let pt = page_index as f64 * PAGE_H;
            let page_svg = render_page(job, &shape_name, &weight_refs, pt);
            pages.push(format!(r#"<g id="page{page_index}">{page_svg}</g>"#));
            page_index += 1;
        }
    }

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
     xmlns="http://www.w3.org/2000/svg">
<style>text {{ font-family: Helvetica, Arial, sans-serif; }}</style>
{body}
</svg>"#
    )
}
