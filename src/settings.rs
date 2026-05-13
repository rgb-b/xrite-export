//! Application settings — persisted to JSON.
//!
//! Includes job presets, step presets, and companion/path configuration.
//! Loaded lazily on first access; cache invalidated on every save.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

// ── Preset types ──────────────────────────────────────────────────────────────

/// Controls which metadata fields are visible in the UI for a given job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldVisibility {
    #[serde(default = "yes")] pub job_name:     bool,
    #[serde(default = "yes")] pub job_number:   bool,
    #[serde(default = "yes")] pub customer:     bool,
    #[serde(default = "yes")] pub plate_tech:   bool,
    #[serde(default = "yes")] pub press_system: bool,
    #[serde(default)]         pub esxr_number:  bool,
    #[serde(default = "yes")] pub print_type:   bool,
    #[serde(default = "yes")] pub date:         bool,
    #[serde(default = "yes")] pub set_number:   bool,
    #[serde(default = "yes")] pub inks:         bool,
    #[serde(default = "yes")] pub lpis:         bool,
    #[serde(default = "yes")] pub steps:        bool,
}

fn yes() -> bool { true }

impl Default for FieldVisibility {
    fn default() -> Self {
        Self {
            job_name: true, job_number: true, customer: true,
            plate_tech: true, press_system: true, esxr_number: false,
            print_type: true, date: true, set_number: true,
            inks: true, lpis: true, steps: true,
        }
    }
}

/// How the data entry grid is presented in the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GridLayout {
    /// Shape tabs → LPI tabs → grid. For multi-shape, multi-LPI jobs.
    #[default]
    Tabbed,
    /// Single flat grid, no tab hierarchy. For quick single-combo scans.
    Flat,
}

/// How the export report is laid out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReportLayout {
    #[default]
    Single,
    /// Two sessions side by side / stacked on the same report sheet.
    DualComparison,
}

/// A saved job configuration preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPreset {
    pub name: String,
    #[serde(default)]
    pub fields: FieldVisibility,
    #[serde(default)]
    pub grid_layout: GridLayout,
    #[serde(default)]
    pub report_layout: ReportLayout,
    /// Step preset to activate when this job preset is loaded.
    #[serde(default)]
    pub default_step_preset: Option<String>,
}

/// A named, reusable list of step percentage values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepPreset {
    pub name: String,
    /// e.g. ["100","75","50","25","10","2"]
    pub steps: Vec<String>,
}

impl StepPreset {
    pub fn standard_14() -> Self {
        Self {
            name: "Standard 14".into(),
            steps: ["100","95","90","80","70","60","50","40","30","20","10","5","3","1"]
                .iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn extended_16() -> Self {
        Self {
            name: "Extended 16".into(),
            steps: ["100","95","90","80","70","60","50","40","30","20","10","5","3","1","0.8","0.4"]
                .iter().map(|s| s.to_string()).collect(),
        }
    }
}

// ── Settings root ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_job_presets")]
    pub job_presets: Vec<JobPreset>,

    #[serde(default = "default_step_presets")]
    pub step_presets: Vec<StepPreset>,

    /// Saved dot type options shown in the shape picker (e.g. "CRS", "HD").
    /// New types are appended automatically when the user creates one.
    #[serde(default = "default_dot_types")]
    pub dot_types: Vec<String>,

    /// Saved LPI values shown in the LPI picker (e.g. "150#", "175#").
    /// New values are appended automatically when the user creates one.
    #[serde(default = "default_lpi_values")]
    pub lpi_values: Vec<String>,

    #[serde(default)]
    pub last_session_path: String,

    // Companion (optional — Illustrator PDF bridge on Windows)
    #[serde(default)] pub illustrator_path:    String,
    #[serde(default)] pub ai_template:         String,
    #[serde(default)] pub ai_template_extended: String,
}

fn default_dot_types() -> Vec<String> {
    ["CRS", "HD", "ESXR", "QUA", "SQUA"]
        .iter().map(|s| s.to_string()).collect()
}

fn default_lpi_values() -> Vec<String> {
    ["120#", "150#", "175#", "200#"]
        .iter().map(|s| s.to_string()).collect()
}

fn default_job_presets() -> Vec<JobPreset> {
    vec![
        JobPreset {
            name: "Gradation Strip".into(),
            fields: FieldVisibility {
                job_name: false, job_number: true, customer: true,
                plate_tech: true, press_system: true, esxr_number: false,
                print_type: true, date: true, set_number: true,
                inks: false, lpis: true, steps: true,
            },
            grid_layout: GridLayout::Tabbed,
            report_layout: ReportLayout::Single,
            default_step_preset: Some("Standard 14".into()),
        },
        JobPreset {
            name: "Sample Scan".into(),
            fields: FieldVisibility {
                job_name: true, job_number: true, customer: true,
                plate_tech: true, press_system: true, esxr_number: true,
                print_type: true, date: true, set_number: false,
                inks: true, lpis: false, steps: true,
            },
            grid_layout: GridLayout::Flat,
            report_layout: ReportLayout::Single,
            default_step_preset: None,
        },
    ]
}

fn default_step_presets() -> Vec<StepPreset> {
    vec![StepPreset::standard_14(), StepPreset::extended_16()]
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            job_presets:          default_job_presets(),
            step_presets:         default_step_presets(),
            dot_types:            default_dot_types(),
            lpi_values:           default_lpi_values(),
            last_session_path:    String::new(),
            illustrator_path:     String::new(),
            ai_template:          String::new(),
            ai_template_extended: String::new(),
        }
    }
}

#[allow(dead_code)] // preset lookup helpers — used by future UI wiring
impl Settings {
    pub fn find_step_preset(&self, name: &str) -> Option<&StepPreset> {
        self.step_presets.iter().find(|p| p.name == name)
    }

    pub fn find_job_preset(&self, name: &str) -> Option<&JobPreset> {
        self.job_presets.iter().find(|p| p.name == name)
    }

    /// Flat key/value map for the web API (companion paths + last session).
    pub fn to_flat_map(&self) -> HashMap<String, serde_json::Value> {
        let mut m = HashMap::new();
        m.insert("last_session_path".into(),    self.last_session_path.clone().into());
        m.insert("illustrator_path".into(),     self.illustrator_path.clone().into());
        m.insert("ai_template".into(),          self.ai_template.clone().into());
        m.insert("ai_template_extended".into(), self.ai_template_extended.clone().into());
        m
    }
}

// ── Persistence ───────────────────────────────────────────────────────────────

static CACHE: Lazy<Mutex<Option<Settings>>> = Lazy::new(|| Mutex::new(None));

fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("InkDensityTool").join("settings.json"))
}

/// Load settings (cached after first call).
pub fn load() -> Settings {
    let mut cache = CACHE.lock().unwrap();
    if let Some(ref s) = *cache {
        return s.clone();
    }
    let s = read_from_disk();
    *cache = Some(s.clone());
    s
}

fn read_from_disk() -> Settings {
    let path = match settings_path() {
        Some(p) => p,
        None => return Settings::default(),
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Settings::default(),
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Persist settings and refresh the cache.
pub fn save(settings: &Settings) -> anyhow::Result<()> {
    let path = settings_path().ok_or_else(|| anyhow::anyhow!("Cannot determine config dir"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(settings)?)?;
    *CACHE.lock().unwrap() = Some(settings.clone());
    Ok(())
}

/// Patch a flat key/value pair (used by web API for companion settings).
pub fn patch(key: &str, value: serde_json::Value) {
    let mut s = load();
    match key {
        "last_session_path"    => s.last_session_path    = value.as_str().unwrap_or("").into(),
        "illustrator_path"     => s.illustrator_path     = value.as_str().unwrap_or("").into(),
        "ai_template"          => s.ai_template          = value.as_str().unwrap_or("").into(),
        "ai_template_extended" => s.ai_template_extended = value.as_str().unwrap_or("").into(),
        _ => {}
    }
    let _ = save(&s);
}

/// Read a single flat string setting.
pub fn get_str(key: &str) -> String {
    let s = load();
    match key {
        "last_session_path"    => s.last_session_path,
        "illustrator_path"     => s.illustrator_path,
        "ai_template"          => s.ai_template,
        "ai_template_extended" => s.ai_template_extended,
        _ => String::new(),
    }
}
