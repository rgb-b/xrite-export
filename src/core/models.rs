use serde::{Deserialize, Serialize};

pub const STEP_LABELS_14: &[&str] = &[
    "100", "95", "90", "80", "70", "60", "50", "40", "30", "20", "10", "5", "3", "1",
];

pub const STEP_LABELS_16: &[&str] = &[
    "100", "95", "90", "80", "70", "60", "50", "40", "30", "20", "10", "5", "3", "1", "0.8", "0.4",
];

pub const COLOUR_NAMES: &[&str] = &["C", "M", "Y", "K"];

/// One weight (LPI) table: max density + step readings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightData {
    pub label: String,
    /// 4 max-density readings: [C, M, Y, K]
    pub density: [f64; 4],
    /// [N rows][4 colours] — percentage readings at each step
    pub steps: Vec<[f64; 4]>,
}

impl WeightData {
    pub fn new(label: impl Into<String>, num_steps: usize) -> Self {
        Self {
            label: label.into(),
            density: [0.0; 4],
            steps: vec![[0.0; 4]; num_steps],
        }
    }
}

impl Default for WeightData {
    fn default() -> Self {
        Self::new("", STEP_LABELS_14.len())
    }
}

/// One dot shape containing multiple weight tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeData {
    pub name: String,
    pub weights: Vec<WeightData>,
}

/// Full job configuration — metadata + shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    #[serde(default)]
    pub customer: String,
    #[serde(default = "default_print_type")]
    pub print_type: String,
    #[serde(default)]
    pub stock_desc: String,
    #[serde(default = "default_finish")]
    pub finish: String,
    #[serde(default = "default_dot_shape_type")]
    pub dot_shape_type: String,
    #[serde(default)]
    pub dot_shape_number: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub set_number: String,
    #[serde(default)]
    pub job_number: String,
    #[serde(default = "default_weight_labels")]
    pub weight_labels: Vec<String>,
    #[serde(default = "default_step_labels")]
    pub step_labels: Vec<String>,
    #[serde(default = "default_colour_names")]
    pub colour_names: Vec<String>,
    #[serde(default)]
    pub shapes: Vec<ShapeData>,
}

fn default_print_type() -> String { "CRS".into() }
fn default_finish() -> String { "RP".into() }
fn default_dot_shape_type() -> String { "CRS".into() }
fn default_weight_labels() -> Vec<String> {
    vec!["120#".into(), "150#".into(), "200#".into()]
}
fn default_step_labels() -> Vec<String> {
    STEP_LABELS_14.iter().map(|s| s.to_string()).collect()
}
fn default_colour_names() -> Vec<String> {
    COLOUR_NAMES.iter().map(|s| s.to_string()).collect()
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            customer: String::new(),
            print_type: default_print_type(),
            stock_desc: String::new(),
            finish: default_finish(),
            dot_shape_type: default_dot_shape_type(),
            dot_shape_number: String::new(),
            date: String::new(),
            set_number: String::new(),
            job_number: String::new(),
            weight_labels: default_weight_labels(),
            step_labels: default_step_labels(),
            colour_names: default_colour_names(),
            shapes: Vec::new(),
        }
    }
}

impl JobConfig {
    /// Build the heading string: "{customer} {print_type} {stock_desc} {finish}"
    pub fn heading(&self) -> String {
        [&self.customer, &self.print_type, &self.stock_desc, &self.finish]
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Build the dot shape string: "{dot_shape_type} {dot_shape_number}"
    pub fn dot_shape(&self) -> String {
        [&self.dot_shape_type, &self.dot_shape_number]
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn num_steps(&self) -> usize {
        self.step_labels.len()
    }
}
