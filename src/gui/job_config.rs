//! JobConfigPanel — left panel with job metadata and weight label management.

use crate::core::models::{JobConfig, STEP_LABELS_14, STEP_LABELS_16};

const PRINT_TYPES: &[&str] = &["CRS", "QUA"];
const FINISHES: &[&str] = &["RP", "SP", "CBW SP"];
const DOT_SHAPE_TYPES: &[&str] = &["CRS", "CRY", "HD", "ESXR"];

/// State for the job configuration panel.
pub struct JobConfigState {
    pub customer: String,
    pub print_type_idx: usize,
    pub stock_desc: String,
    pub finish_idx: usize,
    pub dot_shape_type_idx: usize,
    pub dot_shape_number: String,
    pub date: String,
    pub set_number: String,
    pub job_number: String,
    pub weight_labels: Vec<String>,
    pub step_preset: StepPreset,

    /// Signals that weight labels changed this frame
    pub weights_changed: bool,
    /// Signals that step labels changed this frame
    pub steps_changed: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum StepPreset {
    Standard14,
    Extended16,
}

impl JobConfigState {
    pub fn new() -> Self {
        Self {
            customer: String::new(),
            print_type_idx: 0,
            stock_desc: String::new(),
            finish_idx: 0,
            dot_shape_type_idx: 0,
            dot_shape_number: String::new(),
            date: String::new(),
            set_number: String::new(),
            job_number: String::new(),
            weight_labels: vec!["120#".into(), "150#".into(), "200#".into()],
            step_preset: StepPreset::Standard14,
            weights_changed: false,
            steps_changed: false,
        }
    }

    pub fn populate(&mut self, job: &JobConfig) {
        self.customer = job.customer.clone();
        self.print_type_idx = PRINT_TYPES
            .iter()
            .position(|&s| s == job.print_type)
            .unwrap_or(0);
        self.stock_desc = job.stock_desc.clone();
        self.finish_idx = FINISHES
            .iter()
            .position(|&s| s == job.finish)
            .unwrap_or(0);
        self.dot_shape_type_idx = DOT_SHAPE_TYPES
            .iter()
            .position(|&s| s == job.dot_shape_type)
            .unwrap_or(0);
        self.dot_shape_number = job.dot_shape_number.clone();
        self.date = job.date.clone();
        self.set_number = job.set_number.clone();
        self.job_number = job.job_number.clone();
        self.weight_labels = job.weight_labels.clone();
        self.step_preset = if job.step_labels.len() == 16 {
            StepPreset::Extended16
        } else {
            StepPreset::Standard14
        };
    }

    pub fn print_type(&self) -> &str {
        PRINT_TYPES.get(self.print_type_idx).unwrap_or(&"CRS")
    }

    pub fn finish(&self) -> &str {
        FINISHES.get(self.finish_idx).unwrap_or(&"RP")
    }

    pub fn dot_shape_type(&self) -> &str {
        DOT_SHAPE_TYPES.get(self.dot_shape_type_idx).unwrap_or(&"CRS")
    }

    /// Clear all job metadata fields (not weight labels or step preset).
    pub fn clear(&mut self) {
        self.customer.clear();
        self.stock_desc.clear();
        self.dot_shape_number.clear();
        self.date.clear();
        self.set_number.clear();
        self.job_number.clear();
        self.print_type_idx = 0;
        self.finish_idx = 0;
        self.dot_shape_type_idx = 0;
    }

    pub fn step_labels(&self) -> Vec<String> {
        match self.step_preset {
            StepPreset::Standard14 => STEP_LABELS_14.iter().map(|s| s.to_string()).collect(),
            StepPreset::Extended16 => STEP_LABELS_16.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Draw the job config panel. Returns true if anything changed.
pub fn show_job_config(ui: &mut egui::Ui, state: &mut JobConfigState) -> bool {
    let mut changed = false;
    state.weights_changed = false;
    state.steps_changed = false;

    ui.strong("Job Configuration");
    ui.add_space(6.0);

    // Customer
    ui.label("Customer:");
    if ui.text_edit_singleline(&mut state.customer).changed() {
        changed = true;
    }

    ui.add_space(4.0);

    // Heading: print type + stock + finish
    ui.label("Heading:");
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("print_type")
            .width(50.0)
            .selected_text(state.print_type())
            .show_ui(ui, |ui| {
                for (i, &pt) in PRINT_TYPES.iter().enumerate() {
                    if ui.selectable_value(&mut state.print_type_idx, i, pt).changed() {
                        changed = true;
                    }
                }
            });

        if ui
            .add_sized([60.0, 20.0], egui::TextEdit::singleline(&mut state.stock_desc))
            .changed()
        {
            changed = true;
        }

        egui::ComboBox::from_id_salt("finish")
            .width(70.0)
            .selected_text(state.finish())
            .show_ui(ui, |ui| {
                for (i, &f) in FINISHES.iter().enumerate() {
                    if ui.selectable_value(&mut state.finish_idx, i, f).changed() {
                        changed = true;
                    }
                }
            });
    });

    ui.add_space(4.0);

    // Dot Shape
    ui.label("Dot Shape:");
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("dot_shape_type")
            .width(60.0)
            .selected_text(state.dot_shape_type())
            .show_ui(ui, |ui| {
                for (i, &ds) in DOT_SHAPE_TYPES.iter().enumerate() {
                    if ui.selectable_value(&mut state.dot_shape_type_idx, i, ds).changed() {
                        changed = true;
                    }
                }
            });

        if ui
            .add_sized([50.0, 20.0], egui::TextEdit::singleline(&mut state.dot_shape_number))
            .changed()
        {
            changed = true;
        }
    });

    ui.add_space(4.0);

    // Date
    ui.label("Date:");
    if ui.text_edit_singleline(&mut state.date).changed() {
        changed = true;
    }

    ui.add_space(4.0);

    // Set # / Job #
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label("Set #:");
            if ui
                .add_sized([60.0, 20.0], egui::TextEdit::singleline(&mut state.set_number))
                .changed()
            {
                changed = true;
            }
        });
        ui.vertical(|ui| {
            ui.label("Job #:");
            if ui
                .add_sized([60.0, 20.0], egui::TextEdit::singleline(&mut state.job_number))
                .changed()
            {
                changed = true;
            }
        });
    });

    ui.separator();

    // LPIs (weight labels)
    ui.label("LPIs:");

    let mut remove_idx: Option<usize> = None;
    let num_labels = state.weight_labels.len();
    for i in 0..num_labels {
        ui.horizontal(|ui| {
            if ui
                .add_sized([60.0, 20.0], egui::TextEdit::singleline(&mut state.weight_labels[i]))
                .changed()
            {
                changed = true;
                state.weights_changed = true;
            }
            if num_labels > 1 && ui.small_button("x").clicked() {
                remove_idx = Some(i);
            }
        });
    }
    if let Some(idx) = remove_idx {
        state.weight_labels.remove(idx);
        changed = true;
        state.weights_changed = true;
    }
    if ui.button("+ Add LPI").clicked() {
        let n = state.weight_labels.len() + 1;
        state.weight_labels.push(format!("LPI{n}"));
        changed = true;
        state.weights_changed = true;
    }

    ui.separator();

    // Step preset
    ui.label("Steps:");
    let old_preset = state.step_preset;
    ui.radio_value(
        &mut state.step_preset,
        StepPreset::Standard14,
        "Standard (100 -> 1)",
    );
    ui.radio_value(
        &mut state.step_preset,
        StepPreset::Extended16,
        "Extended (100 -> 0.4)",
    );
    if state.step_preset != old_preset {
        changed = true;
        state.steps_changed = true;
    }

    changed
}
