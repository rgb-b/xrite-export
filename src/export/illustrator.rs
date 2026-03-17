//! Illustrator PDF export via ExtendScript subprocess.
//!
//! Flow per dot shape:
//! 1. Chunk the shape's LPIs (weights) into groups of up to 3.
//! 2. For each chunk, pick the matching template (1LPI / 2LPI / 3LPI).
//! 3. Build a placeholder → value dict for the chunk.
//! 4. Render runner.jsx with all values substituted.
//! 5. Write a temp .jsx and call Illustrator.exe in batch mode.
//! 6. After all shapes/chunks, merge all PDFs into one final file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::core::models::JobConfig;
use crate::export::pdf_merge;
use crate::export::placeholders::{build_placeholders, chunk_weights};
use crate::settings;

/// Runner.jsx template — embedded at compile time.
const RUNNER_JSX: &str = include_str!("../../assets/runner.jsx");

/// Common Illustrator install paths to probe (Windows).
const ILLUSTRATOR_SEARCH_PATHS: &[&str] = &[
    r"C:\Program Files\Adobe\Adobe Illustrator 2025\Support Files\Contents\Windows\Illustrator.exe",
    r"C:\Program Files\Adobe\Adobe Illustrator 2024\Support Files\Contents\Windows\Illustrator.exe",
    r"C:\Program Files\Adobe\Adobe Illustrator 2023\Support Files\Contents\Windows\Illustrator.exe",
    r"C:\Program Files\Adobe\Adobe Illustrator 2022\Support Files\Contents\Windows\Illustrator.exe",
    r"C:\Program Files\Adobe\Adobe Illustrator 2021\Support Files\Contents\Windows\Illustrator.exe",
    r"C:\Program Files\Adobe\Adobe Illustrator CC 2020\Support Files\Contents\Windows\Illustrator.exe",
];

pub fn find_illustrator() -> Option<String> {
    let saved = settings::get_str("illustrator_path");
    if !saved.is_empty() && Path::new(&saved).is_file() {
        return Some(saved);
    }
    for candidate in ILLUSTRATOR_SEARCH_PATHS {
        if Path::new(candidate).is_file() {
            return Some(candidate.to_string());
        }
    }
    None
}

fn jsx_escape_path(p: &str) -> String {
    p.replace('\\', "/").replace('"', "\\\"")
}

fn jsx_escape_value(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render_jsx(
    placeholders: &HashMap<String, String>,
    template_ai_path: &str,
    out_pdf_path: &str,
) -> String {
    let mut jsx_source = RUNNER_JSX.to_string();

    // Build the replacements object literal: {"<<KEY>>": "value", ...}
    let entries: Vec<String> = placeholders
        .iter()
        .map(|(key, value)| format!("        \"{}\": \"{}\"", key, jsx_escape_value(value)))
        .collect();
    let replacements_js = format!("{{\n{}\n    }}", entries.join(",\n"));
    jsx_source = jsx_source.replace("<<REPLACEMENTS_DICT>>", &replacements_js);

    // Inject path tokens
    jsx_source = jsx_source.replace("<<TEMPLATE_PATH>>", &jsx_escape_path(template_ai_path));
    jsx_source = jsx_source.replace("<<OUTPUT_PDF>>", &jsx_escape_path(out_pdf_path));

    jsx_source
}

pub fn export_pdf(job: &JobConfig, output_path: &Path) -> Result<()> {
    let illustrator_exe = find_illustrator()
        .context("Illustrator.exe not found. Please set the path in Settings.")?;

    let num_steps = job.num_steps();
    let suffix = if num_steps > 14 { "_extended" } else { "" };
    let template_path = settings::get_str(&format!("ai_template{suffix}"));
    if template_path.is_empty() || !Path::new(&template_path).is_file() {
        bail!("Illustrator template not set. Please set the template path in Settings.");
    }

    if job.shapes.is_empty() {
        bail!("No shapes to export.");
    }
    for shape in &job.shapes {
        if shape.weights.is_empty() {
            bail!("Shape '{}' has no LPIs. Add at least one before exporting.", shape.name);
        }
    }

    let tmp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let mut all_pdfs: Vec<PathBuf> = Vec::new();

    for shape in &job.shapes {
        let chunks = chunk_weights(&shape.weights, 3);

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let safe_name: String = shape
                .name
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
                .collect();
            let out_pdf = tmp_dir.path().join(format!("{safe_name}_chunk{chunk_idx}.pdf"));

            let placeholders = build_placeholders(job, shape, chunk);
            let jsx_source = render_jsx(&placeholders, &template_path, &out_pdf.to_string_lossy());

            let jsx_path = tmp_dir.path().join(format!("{safe_name}_chunk{chunk_idx}.jsx"));
            std::fs::write(&jsx_path, &jsx_source)
                .with_context(|| format!("Failed to write JSX: {}", jsx_path.display()))?;

            let result = Command::new(&illustrator_exe)
                .args(["/b", &jsx_path.to_string_lossy()])
                .output();

            match result {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let mut msg = format!(
                            "Illustrator returned exit code {:?} for shape '{}' chunk {}",
                            output.status.code(),
                            shape.name,
                            chunk_idx + 1
                        );
                        if !stderr.is_empty() {
                            msg.push('\n');
                            msg.push_str(stderr.trim());
                        }
                        bail!("{msg}");
                    }
                }
                Err(e) => {
                    bail!(
                        "Failed to run Illustrator for shape '{}' chunk {}: {e}",
                        shape.name,
                        chunk_idx + 1
                    );
                }
            }

            if !out_pdf.is_file() {
                bail!(
                    "Illustrator did not produce a PDF for shape '{}' chunk {}",
                    shape.name,
                    chunk_idx + 1
                );
            }

            all_pdfs.push(out_pdf);
        }
    }

    // Merge all PDFs
    pdf_merge::merge_pdfs(&all_pdfs, output_path)?;

    Ok(())
}
