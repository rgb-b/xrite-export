//! LibreOffice PDF export backend — Linux substitute for Illustrator.
//!
//! Since there are no Rust UNO bindings, we spawn a small Python helper script
//! that does the UNO work. Rust passes it the placeholder dict as JSON.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::core::models::JobConfig;
use crate::export::pdf_merge;
use crate::export::placeholders::{build_placeholders, chunk_weights};
use crate::settings;

/// Embedded Python helper script for UNO bridge work.
const UNO_HELPER_PY: &str = include_str!("../../assets/lo_uno_helper.py");

pub fn find_libreoffice() -> Option<String> {
    let saved = settings::get_str("libreoffice_path");
    if !saved.is_empty() && Path::new(&saved).is_file() {
        return Some(saved);
    }
    for name in &["libreoffice", "soffice"] {
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }
    None
}

pub fn export_pdf(job: &JobConfig, output_path: &Path) -> Result<()> {
    let lo_exe = find_libreoffice()
        .context("LibreOffice not found.\nInstall with:  sudo apt install libreoffice python3-uno")?;

    let num_steps = job.num_steps();
    let suffix = if num_steps > 14 { "_extended" } else { "" };
    let template_path = settings::get_str(&format!("ai_template{suffix}"));
    if template_path.is_empty() || !Path::new(&template_path).is_file() {
        bail!("Template not set. Please set the template path in Settings.");
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

    // Write the Python UNO helper script to temp
    let helper_path = tmp_dir.path().join("lo_uno_helper.py");
    std::fs::write(&helper_path, UNO_HELPER_PY)
        .context("Failed to write UNO helper script")?;

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

            let placeholders = build_placeholders(job, shape, &chunk);
            let placeholders_json = serde_json::to_string(&placeholders)
                .context("Failed to serialize placeholders")?;

            // Write placeholders JSON to temp file
            let json_path = tmp_dir.path().join(format!("{safe_name}_chunk{chunk_idx}.json"));
            std::fs::write(&json_path, &placeholders_json)
                .context("Failed to write placeholders JSON")?;

            // Run the Python UNO helper
            let result = Command::new("python3")
                .args([
                    helper_path.to_str().unwrap(),
                    &lo_exe,
                    &template_path,
                    json_path.to_str().unwrap(),
                    out_pdf.to_str().unwrap(),
                ])
                .output()
                .with_context(|| {
                    format!(
                        "Failed to run UNO helper for shape '{}' chunk {}",
                        shape.name,
                        chunk_idx + 1
                    )
                })?;

            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                bail!(
                    "LibreOffice export failed for shape '{}' chunk {}:\n{}",
                    shape.name,
                    chunk_idx + 1,
                    stderr.trim()
                );
            }

            if !out_pdf.is_file() {
                bail!(
                    "LibreOffice did not produce a PDF for shape '{}' chunk {}",
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
