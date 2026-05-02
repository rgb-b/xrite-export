//! Core data model for the Ink Density Tool.
//!
//! `JobConfig` is the root — everything the user records for one job.
//! It serialises to/from JSON for session save/load.

use serde::{Deserialize, Serialize};

// ── Ink ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InkKind {
    Cyan,
    Magenta,
    Yellow,
    Black,
    White,
    Spot,
}

impl InkKind {
    /// Whether this ink kind is included in the CMYK deviation average.
    pub fn in_deviation_average(&self) -> bool {
        matches!(self, InkKind::Cyan | InkKind::Magenta | InkKind::Yellow | InkKind::Black)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ink {
    pub kind: InkKind,
    /// Display name: "C", "M", "Y", "K", "W", "PMS 485", etc.
    pub name: String,
}

impl Ink {
    pub fn cyan()    -> Self { Self { kind: InkKind::Cyan,    name: "C".into() } }
    pub fn magenta() -> Self { Self { kind: InkKind::Magenta, name: "M".into() } }
    pub fn yellow()  -> Self { Self { kind: InkKind::Yellow,  name: "Y".into() } }
    pub fn black()   -> Self { Self { kind: InkKind::Black,   name: "K".into() } }
    pub fn white()   -> Self { Self { kind: InkKind::White,   name: "W".into() } }
    pub fn spot(name: impl Into<String>) -> Self {
        Self { kind: InkKind::Spot, name: name.into() }
    }
}

/// Default ink set: CMYK.
pub fn default_inks() -> Vec<Ink> {
    vec![Ink::cyan(), Ink::magenta(), Ink::yellow(), Ink::black()]
}

// ── Weight (LPI reading) ──────────────────────────────────────────────────────

/// One LPI reading: max density + per-step values.
/// Column count matches the number of inks in the parent `JobConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightData {
    /// Lines-per-inch label, e.g. "120#", "150#", "200#".
    pub lpi: String,
    /// Max density per ink: one value per ink channel.
    pub density: Vec<f64>,
    /// Step readings: `steps[step_idx][ink_idx]`.
    pub steps: Vec<Vec<f64>>,
}

impl WeightData {
    pub fn new(lpi: impl Into<String>, num_inks: usize, num_steps: usize) -> Self {
        Self {
            lpi: lpi.into(),
            density: vec![0.0; num_inks],
            steps: vec![vec![0.0; num_inks]; num_steps],
        }
    }

    /// Resize density and step vectors to match a new ink count.
    /// New columns are zero-filled; excess columns are dropped.
    pub fn resize_inks(&mut self, num_inks: usize) {
        self.density.resize(num_inks, 0.0);
        for row in &mut self.steps {
            row.resize(num_inks, 0.0);
        }
    }

    /// Resize step rows to match a new step count.
    /// New rows are zero-filled; excess rows are dropped.
    pub fn resize_steps(&mut self, num_steps: usize, num_inks: usize) {
        self.steps.resize_with(num_steps, || vec![0.0; num_inks]);
    }
}

// ── Shape (dot shape) ─────────────────────────────────────────────────────────

/// One dot shape, containing one or more LPI weight readings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeData {
    /// Dot type abbreviation: "CRS", "HD", "ESXR", "QUA", etc.
    pub dot_type: String,
    /// Dot number / ruling: "501", "16", "01", etc.
    pub dot_number: String,
    /// LPI readings for this shape. Gradation strip jobs typically have 3+;
    /// sample scan jobs typically have 1.
    pub weights: Vec<WeightData>,
}

impl ShapeData {
    pub fn display_name(&self) -> String {
        if self.dot_number.is_empty() {
            self.dot_type.clone()
        } else {
            format!("{} {}", self.dot_type, self.dot_number)
        }
    }
}

// ── JobConfig ─────────────────────────────────────────────────────────────────

/// Full job — metadata, ink configuration, and all shape/LPI readings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    /// Name of the preset used to create this job (informational).
    #[serde(default)]
    pub preset_name: String,

    // ── Metadata fields ───────────────────────────────────────────────────────
    // All optional in the sense that any can be empty string.
    // Which fields are shown in the UI is controlled by the active JobPreset.

    #[serde(default)]
    pub job_name: String,
    #[serde(default)]
    pub job_number: String,
    #[serde(default)]
    pub customer: String,

    /// Plate technology: "CRS" (Crystal) | "QUA" (Quartz) | ""
    #[serde(default)]
    pub plate_tech: String,

    /// Press system: "XPS" | "ITP" | ""
    #[serde(default)]
    pub press_system: String,

    /// Optional screening spec number, e.g. "3245".
    #[serde(default)]
    pub esxr_number: String,

    /// Print type: "RP" | "SP" | "CBW SP" | ""
    #[serde(default)]
    pub print_type: String,

    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub set_number: String,

    // ── Scan configuration ────────────────────────────────────────────────────

    /// Active ink channels — drives the number of data columns in the grid.
    #[serde(default = "default_inks")]
    pub inks: Vec<Ink>,

    /// Step percentage labels in scan order, e.g. ["100","95","90",...,"1"].
    #[serde(default = "default_step_labels")]
    pub step_labels: Vec<String>,

    /// Dot shapes, each with one or more LPI weight readings.
    #[serde(default)]
    pub shapes: Vec<ShapeData>,
}

fn default_step_labels() -> Vec<String> {
    // Standard 14-step as default
    ["100","95","90","80","70","60","50","40","30","20","10","5","3","1"]
        .iter().map(|s| s.to_string()).collect()
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            preset_name:  String::new(),
            job_name:     String::new(),
            job_number:   String::new(),
            customer:     String::new(),
            plate_tech:   String::new(),
            press_system: String::new(),
            esxr_number:  String::new(),
            print_type:   String::new(),
            date:         String::new(),
            set_number:   String::new(),
            inks:         default_inks(),
            step_labels:  default_step_labels(),
            shapes:       Vec::new(),
        }
    }
}

impl JobConfig {
    /// Auto-assembled heading: "Customer — CRS XPS ESXR — RP"
    /// Only includes non-empty components.
    pub fn heading(&self) -> String {
        let spec_parts: Vec<&str> = [
            self.plate_tech.as_str(),
            self.press_system.as_str(),
            self.esxr_number.as_str(),
        ]
        .iter()
        .copied()
        .filter(|s| !s.is_empty())
        .collect();

        let parts: Vec<String> = [
            (!self.customer.is_empty()).then(|| self.customer.clone()),
            (!spec_parts.is_empty()).then(|| spec_parts.join(" ")),
            (!self.print_type.is_empty()).then(|| self.print_type.clone()),
        ]
        .into_iter()
        .flatten()
        .collect();

        parts.join(" — ")
    }

    pub fn num_inks(&self) -> usize  { self.inks.len() }
    pub fn num_steps(&self) -> usize { self.step_labels.len() }

    /// Indices of inks included in the deviation average (CMYK only).
    pub fn deviation_ink_indices(&self) -> Vec<usize> {
        self.inks
            .iter()
            .enumerate()
            .filter(|(_, ink)| ink.kind.in_deviation_average())
            .map(|(i, _)| i)
            .collect()
    }

    /// Compute the CMYK average for one step row across all deviation inks.
    /// Returns `None` if there are no deviation inks or all values are zero.
    pub fn step_average(&self, row_values: &[f64]) -> Option<f64> {
        let indices = self.deviation_ink_indices();
        if indices.is_empty() { return None; }
        let sum: f64 = indices.iter().map(|&i| row_values.get(i).copied().unwrap_or(0.0)).sum();
        Some(sum / indices.len() as f64)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_all_fields() {
        let mut job = JobConfig::default();
        job.customer     = "Acme".into();
        job.plate_tech   = "CRS".into();
        job.press_system = "XPS".into();
        job.print_type   = "RP".into();
        assert_eq!(job.heading(), "Acme — CRS XPS — RP");
    }

    #[test]
    fn heading_with_esxr() {
        let mut job = JobConfig::default();
        job.customer     = "TestCo".into();
        job.plate_tech   = "QUA".into();
        job.press_system = "XPS".into();
        job.esxr_number  = "3245".into();
        job.print_type   = "SP".into();
        assert_eq!(job.heading(), "TestCo — QUA XPS 3245 — SP");
    }

    #[test]
    fn heading_partial() {
        let mut job = JobConfig::default();
        job.customer   = "Acme".into();
        job.print_type = "RP".into();
        assert_eq!(job.heading(), "Acme — RP");
    }

    #[test]
    fn deviation_excludes_spots() {
        let mut job = JobConfig::default();
        job.inks.push(Ink::spot("PMS 485"));
        // CMYK = indices 0-3, spot = index 4
        let indices = job.deviation_ink_indices();
        assert_eq!(indices, vec![0, 1, 2, 3]);
        assert!(!indices.contains(&4));
    }

    #[test]
    fn weight_resize() {
        let mut w = WeightData::new("150#", 4, 14);
        assert_eq!(w.density.len(), 4);
        w.resize_inks(5);
        assert_eq!(w.density.len(), 5);
        assert_eq!(w.steps[0].len(), 5);
    }
}
