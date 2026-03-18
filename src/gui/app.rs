//! Main application window — egui/eframe based.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::core::models::{JobConfig, ShapeData, WeightData};
use crate::core::session;
use crate::gui::job_config::{self, JobConfigState};
use crate::gui::shape_tabs::{self, ShapeNotebookState};
use crate::settings;

/// Return the settings value for `key`, or `default` if empty/missing.
fn xcm_or(key: &str, default: &str) -> String {
    let v = settings::get_str(key);
    if v.is_empty() { default.to_string() } else { v }
}

/// Load a string-vec setting, falling back to `defaults` if absent/empty.
fn dd_or(key: &str, defaults: &[&str]) -> Vec<String> {
    let v = settings::get_string_vec(key);
    if v.is_empty() { defaults.iter().map(|s| s.to_string()).collect() } else { v }
}

/// Export status shared between main thread and background export threads.
#[derive(Clone, Default)]
struct ExportStatus {
    message: String,
    is_error: bool,
    busy: bool,
}

/// Deferred keyboard action — collected inside `ctx.input()` to avoid borrow issues,
/// then dispatched after all menu/UI code has finished.
enum KeyAction { None, New, Open, Save }

pub struct InkDensityApp {
    config_state: JobConfigState,
    shape_state: ShapeNotebookState,
    current_path: Option<PathBuf>,
    dirty: bool,
    status: String,
    export_status: Arc<Mutex<ExportStatus>>,

    // Close-guard dialog
    show_unsaved_close_dialog: bool,

    // Dialog state
    show_add_shape_dialog: bool,
    new_shape_name: String,
    show_rename_dialog: bool,
    rename_shape_name: String,
    rename_shape_idx: usize,

    // Templates dialog
    show_templates_dialog: bool,
    tmpl_standard: String,
    tmpl_extended: String,
    tmpl_excel: String,
    tmpl_libreoffice: String,

    // Dropdown options dialog
    show_dropdowns_dialog: bool,
    dd_print_types: Vec<String>,
    dd_finishes: Vec<String>,
    dd_dot_shape_types: Vec<String>,

    // Cell Mapping dialog
    show_cell_mapping_dialog: bool,
    xcm_title_col:           String,
    xcm_date_col:            String,
    xcm_step_start_row:      String,
    xcm_data_col_c:          String,
    xcm_data_col_m:          String,
    xcm_data_col_y:          String,
    xcm_data_col_k:          String,
    xcm_label_col:           String,
    xcm_dot_shape_col:       String,
    xcm_gap_t1_to_t2:        String,
    xcm_density_row_offset:  String,
    xcm_title_t2_row_offset: String,
}

impl InkDensityApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            config_state: JobConfigState::new(),
            shape_state: ShapeNotebookState::new(),
            current_path: None,
            dirty: false,
            status: "Ready".into(),
            export_status: Arc::new(Mutex::new(ExportStatus::default())),
            show_unsaved_close_dialog: false,
            show_add_shape_dialog: false,
            new_shape_name: String::new(),
            show_rename_dialog: false,
            rename_shape_name: String::new(),
            rename_shape_idx: 0,
            show_templates_dialog: false,
            tmpl_standard: settings::get_str("ai_template"),
            tmpl_extended: settings::get_str("ai_template_extended"),
            tmpl_excel: settings::get_str("excel_template"),
            tmpl_libreoffice: settings::get_str("libreoffice_path"),
            show_dropdowns_dialog: false,
            dd_print_types:     dd_or("dropdown_print_types",     &["CRS", "QUA"]),
            dd_finishes:        dd_or("dropdown_finishes",        &["RP", "SP", "CBW SP"]),
            dd_dot_shape_types: dd_or("dropdown_dot_shape_types", &["CRS", "CRY", "HD", "ESXR"]),
            show_cell_mapping_dialog: false,
            xcm_title_col:           xcm_or("xcm_title_col",           "A"),
            xcm_date_col:            xcm_or("xcm_date_col",            "I"),
            xcm_step_start_row:      xcm_or("xcm_step_start_row",      "4"),
            xcm_data_col_c:          xcm_or("xcm_data_col_c",          "B"),
            xcm_data_col_m:          xcm_or("xcm_data_col_m",          "C"),
            xcm_data_col_y:          xcm_or("xcm_data_col_y",          "D"),
            xcm_data_col_k:          xcm_or("xcm_data_col_k",          "E"),
            xcm_label_col:           xcm_or("xcm_label_col",           "A"),
            xcm_dot_shape_col:       xcm_or("xcm_dot_shape_col",       "I"),
            xcm_gap_t1_to_t2:        xcm_or("xcm_gap_t1_to_t2",        "4"),
            xcm_density_row_offset:  xcm_or("xcm_density_row_offset",  "1"),
            xcm_title_t2_row_offset: xcm_or("xcm_title_t2_row_offset", "3"),
        };
        app.load_initial_session();
        app
    }

    fn load_initial_session(&mut self) {
        let last = settings::get_str("last_session_path");
        if !last.is_empty() {
            if let Ok(job) = session::load_session(std::path::Path::new(&last)) {
                self.populate_from_job(&job);
                self.current_path = Some(PathBuf::from(&last));
                self.status = format!("Loaded: {last}");
                return;
            }
        }
        self.new_job();
    }

    fn new_job(&mut self) {
        let step_labels = self.config_state.step_labels();
        let weight_labels = settings::get_string_vec("default_weight_labels");
        let weight_labels = if weight_labels.is_empty() {
            vec!["120#".into(), "150#".into(), "200#".into()]
        } else {
            weight_labels
        };

        let job = JobConfig {
            weight_labels: weight_labels.clone(),
            step_labels: step_labels.clone(),
            shapes: vec![ShapeData {
                name: "Shape 1".into(),
                weights: weight_labels
                    .iter()
                    .map(|lbl| WeightData::new(lbl, step_labels.len()))
                    .collect(),
            }],
            ..Default::default()
        };
        self.populate_from_job(&job);
        self.current_path = None;
        self.dirty = false;
        self.status = "New job".into();
    }

    fn populate_from_job(&mut self, job: &JobConfig) {
        self.config_state.populate(job);
        self.shape_state
            .populate(&job.shapes, &job.weight_labels, &job.step_labels);
    }

    fn collect_job(&self) -> JobConfig {
        let shapes = self.shape_state.get_all_shapes();
        JobConfig {
            customer: self.config_state.customer.clone(),
            print_type: self.config_state.print_type().to_string(),
            stock_desc: self.config_state.stock_desc.clone(),
            finish: self.config_state.finish().to_string(),
            dot_shape_type: self.config_state.dot_shape_type().to_string(),
            dot_shape_number: self.config_state.dot_shape_number.clone(),
            date: self.config_state.date.clone(),
            set_number: self.config_state.set_number.clone(),
            job_number: self.config_state.job_number.clone(),
            weight_labels: self.config_state.weight_labels.clone(),
            step_labels: self.config_state.step_labels(),
            shapes,
            ..Default::default()
        }
    }

    fn save_session(&mut self) {
        if let Some(ref path) = self.current_path {
            let job = self.collect_job();
            match session::save_session(&job, path) {
                Ok(()) => {
                    self.dirty = false;
                    self.status = format!("Saved: {}", path.display());
                }
                Err(e) => {
                    self.status = format!("Save error: {e}");
                }
            }
        } else {
            self.save_as();
        }
    }

    fn save_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Ink Density Session", &["json"])
            .add_filter("All Files", &["*"])
            .save_file()
        {
            self.current_path = Some(path.clone());
            settings::set_str("last_session_path", &path.to_string_lossy());
            self.save_session();
        }
    }

    fn open_session(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Ink Density Session", &["json"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            match session::load_session(&path) {
                Ok(job) => {
                    self.populate_from_job(&job);
                    self.current_path = Some(path.clone());
                    self.dirty = false;
                    settings::set_str("last_session_path", &path.to_string_lossy());
                    self.status = format!("Opened: {}", path.display());
                }
                Err(e) => {
                    self.status = format!("Open error: {e}");
                }
            }
        }
    }

    fn export_pdf(&mut self) {
        let job = self.collect_job();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .save_file()
        {
            self.status = "Exporting PDF...".into();
            let status = self.export_status.clone();
            {
                let mut s = status.lock().unwrap();
                s.busy = true;
                s.message.clear();
            }

            std::thread::spawn(move || {
                let result = if cfg!(target_os = "windows") {
                    crate::export::illustrator::export_pdf(&job, &path)
                } else {
                    crate::export::libreoffice::export_pdf(&job, &path)
                };
                let mut s = status.lock().unwrap();
                s.busy = false;
                match result {
                    Ok(()) => {
                        s.message = format!("PDF exported: {}", path.display());
                        s.is_error = false;
                    }
                    Err(e) => {
                        s.message = format!("Export error: {e}");
                        s.is_error = true;
                    }
                }
            });
        }
    }

    fn export_excel(&mut self) {
        let job = self.collect_job();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Excel Workbook", &["xlsx"])
            .save_file()
        {
            self.status = "Exporting Excel...".into();
            let status = self.export_status.clone();
            {
                let mut s = status.lock().unwrap();
                s.busy = true;
                s.message.clear();
            }

            std::thread::spawn(move || {
                let result = crate::export::excel::export_excel(&job, &path);
                let mut s = status.lock().unwrap();
                s.busy = false;
                match result {
                    Ok(()) => {
                        s.message = format!("Excel exported: {}", path.display());
                        s.is_error = false;
                    }
                    Err(e) => {
                        s.message = format!("Export error: {e}");
                        s.is_error = true;
                    }
                }
            });
        }
    }

    fn fill_example_data(&mut self) {
        let step_labels = self.config_state.step_labels();
        let weight_labels = vec!["120#".into(), "150#".into(), "200#".into()];

        let steps_c = [100.0, 93.2, 86.5, 74.1, 62.8, 52.3, 42.0, 32.7, 23.9, 15.4, 7.8, 3.9, 2.3, 1.1];
        let steps_m = [100.0, 94.1, 87.3, 75.6, 64.2, 53.8, 43.5, 34.1, 25.2, 16.8, 8.5, 4.2, 2.5, 1.3];
        let steps_y = [100.0, 92.7, 85.9, 73.4, 61.5, 51.0, 40.8, 31.6, 22.7, 14.2, 6.9, 3.4, 2.0, 0.9];
        let steps_k = [100.0, 93.8, 86.9, 74.8, 63.1, 52.7, 42.3, 33.0, 24.3, 15.9, 8.1, 4.0, 2.4, 1.2];

        let num_example_steps = step_labels.len();
        let make_steps = |c_off: f64, m_off: f64| -> Vec<[f64; 4]> {
            (0..num_example_steps)
                .map(|i| {
                    if i < steps_c.len() {
                        [
                            (steps_c[i] + c_off * 10.0).round() / 10.0,
                            (steps_m[i] + m_off * 10.0).round() / 10.0,
                            (steps_y[i] - c_off * 5.0).round() / 10.0,
                            (steps_k[i] + m_off * 5.0).round() / 10.0,
                        ]
                    } else {
                        [0.0; 4]
                    }
                })
                .collect()
        };

        let example_weights = vec![
            WeightData {
                label: "120#".into(),
                density: [2.11, 1.80, 1.66, 1.79],
                steps: make_steps(0.0, 0.0),
            },
            WeightData {
                label: "150#".into(),
                density: [2.08, 1.77, 1.63, 1.75],
                steps: make_steps(0.3, -0.2),
            },
            WeightData {
                label: "200#".into(),
                density: [2.05, 1.74, 1.60, 1.72],
                steps: make_steps(0.6, -0.4),
            },
        ];

        let job = JobConfig {
            customer: "Test Customer".into(),
            print_type: "CRS".into(),
            stock_desc: "XPS".into(),
            finish: "CBW SP".into(),
            dot_shape_type: "CRS".into(),
            dot_shape_number: "01".into(),
            date: "24-02-2026".into(),
            set_number: "01".into(),
            job_number: "J001".into(),
            weight_labels,
            step_labels,
            shapes: vec![ShapeData {
                name: "HD 16".into(),
                weights: example_weights,
            }],
            ..Default::default()
        };
        self.populate_from_job(&job);
        self.dirty = true;
        self.status = "Example data loaded".into();
    }
}

impl eframe::App for InkDensityApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Unsaved-changes close guard ──────────────────────────────────────
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.show_unsaved_close_dialog = true;
        }

        // ── Window title reflects dirty state ────────────────────────────────
        let title = if self.dirty {
            "Ink Density Tool *".to_string()
        } else {
            "Ink Density Tool".to_string()
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        // ── Check export status from background thread ───────────────────────
        {
            let s = self.export_status.lock().unwrap();
            if !s.message.is_empty() {
                self.status = s.message.clone();
            }
            if s.busy {
                ctx.request_repaint();
            }
        }

        // ── Keyboard shortcuts ────────────────────────────────────────────────
        // Captured inside ctx.input() to avoid borrow conflicts, then dispatched below.
        let kb_action = ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                KeyAction::New
            } else if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                KeyAction::Open
            } else if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                KeyAction::Save
            } else {
                KeyAction::None
            }
        });
        match kb_action {
            KeyAction::New  => self.new_job(),
            KeyAction::Open => self.open_session(),
            KeyAction::Save => self.save_session(),
            KeyAction::None => {}
        }

        // ── Top menu bar ──────────────────────────────────────────────────────
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New (Ctrl+N)").clicked() {
                        self.new_job();
                        ui.close_menu();
                    }
                    if ui.button("Open... (Ctrl+O)").clicked() {
                        self.open_session();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Save (Ctrl+S)").clicked() {
                        self.save_session();
                        ui.close_menu();
                    }
                    if ui.button("Save As...").clicked() {
                        self.save_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Fill Example Data").clicked() {
                        self.fill_example_data();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Export", |ui| {
                    if ui.button("Export -> PDF").clicked() {
                        self.export_pdf();
                        ui.close_menu();
                    }
                    if ui.button("Export -> Excel").clicked() {
                        self.export_excel();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Clear", |ui| {
                    if ui.button("Clear Current Page").clicked() {
                        self.shape_state.clear_current_weight();
                        self.dirty = true;
                        ui.close_menu();
                    }
                    if ui.button("Clear All Pages").clicked() {
                        self.shape_state.clear_all_weights();
                        self.dirty = true;
                        ui.close_menu();
                    }
                    if ui.button("Clear All").clicked() {
                        self.shape_state.clear_all_weights();
                        self.config_state.clear();
                        self.dirty = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Settings", |ui| {
                    if ui.button("Illustrator Path...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Executable", &["exe"])
                            .add_filter("All Files", &["*"])
                            .pick_file()
                        {
                            settings::set_str("illustrator_path", &path.to_string_lossy());
                            self.status = format!("Illustrator path set: {}", path.display());
                        }
                        ui.close_menu();
                    }
                    if ui.button("Templates...").clicked() {
                        self.show_templates_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button("Dropdown Options...").clicked() {
                        self.show_dropdowns_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button("Cell Mapping...").clicked() {
                        self.show_cell_mapping_dialog = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("Export Builder Script…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("build_ai_template.jsx")
                            .add_filter("ExtendScript", &["jsx"])
                            .add_filter("All Files", &["*"])
                            .save_file()
                        {
                            match std::fs::write(&path, crate::export::illustrator::builder_jsx()) {
                                Ok(_) => self.status = format!("Builder script saved to {}", path.display()),
                                Err(e) => self.status = format!("Failed to save script: {e}"),
                            }
                        }
                        ui.close_menu();
                    }
                });
            });
        });

        // ── Status bar ────────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Status: {}", self.status));
            });
        });

        // ── Left panel — job config ───────────────────────────────────────────
        egui::SidePanel::left("job_config")
            .default_width(200.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if job_config::show_job_config(ui, &mut self.config_state) {
                        self.dirty = true;
                    }

                    // Propagate weight/step changes to shape notebook
                    if self.config_state.weights_changed {
                        self.shape_state
                            .update_weight_labels(&self.config_state.weight_labels);
                    }
                    if self.config_state.steps_changed {
                        self.shape_state
                            .update_step_labels(&self.config_state.step_labels());
                    }
                });
            });

        // ── Central panel — shape tabs + toolbar ──────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("+ Add Shape").clicked() {
                    self.show_add_shape_dialog = true;
                    self.new_shape_name = format!("Shape {}", self.shape_state.tabs.len() + 1);
                }
                if ui.button("Rename").clicked() {
                    if !self.shape_state.tabs.is_empty() {
                        self.rename_shape_idx = self.shape_state.selected_shape;
                        self.rename_shape_name =
                            self.shape_state.tabs[self.rename_shape_idx].name.clone();
                        self.show_rename_dialog = true;
                    }
                }
                if ui.button("Remove").clicked() {
                    if self.shape_state.tabs.len() > 1 {
                        self.shape_state
                            .remove_shape(self.shape_state.selected_shape);
                        self.dirty = true;
                    }
                }
            });

            ui.separator();

            let time = ui.input(|i| i.time);
            if shape_tabs::show_shape_notebook(ui, &mut self.shape_state, time) {
                self.dirty = true;
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Export -> PDF").clicked() {
                        self.export_pdf();
                    }
                    if ui.button("Export -> Excel").clicked() {
                        self.export_excel();
                    }
                });
            });
        });

        // ── Unsaved-changes close dialog ──────────────────────────────────────
        if self.show_unsaved_close_dialog {
            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("You have unsaved changes. Save before closing?");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save & Close").clicked() {
                            self.save_session();
                            self.show_unsaved_close_dialog = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Discard & Close").clicked() {
                            self.dirty = false;
                            self.show_unsaved_close_dialog = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_unsaved_close_dialog = false;
                        }
                    });
                });
        }

        // ── Add shape dialog ──────────────────────────────────────────────────
        if self.show_add_shape_dialog {
            egui::Window::new("New Dot Shape")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Enter dot shape name (e.g. HD 16):");
                    ui.text_edit_singleline(&mut self.new_shape_name);
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            let name = self.new_shape_name.trim().to_string();
                            if !name.is_empty() {
                                self.shape_state.add_shape(&name);
                                self.dirty = true;
                            }
                            self.show_add_shape_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_add_shape_dialog = false;
                        }
                    });
                });
        }

        // ── Rename dialog ─────────────────────────────────────────────────────
        if self.show_rename_dialog {
            egui::Window::new("Rename Shape")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("New name:");
                    ui.text_edit_singleline(&mut self.rename_shape_name);
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            let name = self.rename_shape_name.trim().to_string();
                            if !name.is_empty() {
                                self.shape_state
                                    .rename_shape(self.rename_shape_idx, &name);
                                self.dirty = true;
                            }
                            self.show_rename_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_rename_dialog = false;
                        }
                    });
                });
        }

        // ── Cell Mapping dialog ───────────────────────────────────────────────
        if self.show_cell_mapping_dialog {
            let mut open = true;
            egui::Window::new("Excel Cell Mapping")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Configure which columns and rows data is written to in the Excel template.");
                    ui.add_space(6.0);
                    egui::Grid::new("xcm_grid")
                        .num_columns(3)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Field").strong());
                            ui.label(egui::RichText::new("Value").strong());
                            ui.label(egui::RichText::new("Description").strong());
                            ui.end_row();

                            let rows: &mut [(&str, &str, &mut String)] = &mut [
                                ("Title column",          "xcm_title_col",           &mut self.xcm_title_col),
                                ("Date column",           "xcm_date_col",            &mut self.xcm_date_col),
                                ("CMYK col — C",          "xcm_data_col_c",          &mut self.xcm_data_col_c),
                                ("CMYK col — M",          "xcm_data_col_m",          &mut self.xcm_data_col_m),
                                ("CMYK col — Y",          "xcm_data_col_y",          &mut self.xcm_data_col_y),
                                ("CMYK col — K",          "xcm_data_col_k",          &mut self.xcm_data_col_k),
                                ("Label column",          "xcm_label_col",           &mut self.xcm_label_col),
                                ("Dot shape column",      "xcm_dot_shape_col",       &mut self.xcm_dot_shape_col),
                                ("Step data start row",   "xcm_step_start_row",      &mut self.xcm_step_start_row),
                                ("Section gap (rows)",    "xcm_gap_t1_to_t2",        &mut self.xcm_gap_t1_to_t2),
                                ("Density row offset",    "xcm_density_row_offset",  &mut self.xcm_density_row_offset),
                                ("Section 2 title offset","xcm_title_t2_row_offset", &mut self.xcm_title_t2_row_offset),
                            ];
                            let descriptions = [
                                "Column for job title / customer",
                                "Column for print date",
                                "Column for Cyan readings",
                                "Column for Magenta readings",
                                "Column for Yellow readings",
                                "Column for Black readings",
                                "Column for weight label (footer row)",
                                "Column for dot shape (footer row)",
                                "First row of tint scale data",
                                "Rows between end of table 1 and start of table 2 step data",
                                "Rows above step start where density row sits (default 1)",
                                "Rows above section 2 step start where section 2 title sits (default 3)",
                            ];
                            for (i, (label, _key, val)) in rows.iter_mut().enumerate() {
                                ui.label(*label);
                                ui.add_sized([60.0, 20.0], egui::TextEdit::singleline(*val));
                                ui.label(descriptions[i]);
                                ui.end_row();
                            }
                        });
                    ui.add_space(4.0);
                    if ui.button("Save & Close").clicked() {
                        settings::set_str("xcm_title_col",           &self.xcm_title_col);
                        settings::set_str("xcm_date_col",            &self.xcm_date_col);
                        settings::set_str("xcm_data_col_c",          &self.xcm_data_col_c);
                        settings::set_str("xcm_data_col_m",          &self.xcm_data_col_m);
                        settings::set_str("xcm_data_col_y",          &self.xcm_data_col_y);
                        settings::set_str("xcm_data_col_k",          &self.xcm_data_col_k);
                        settings::set_str("xcm_label_col",           &self.xcm_label_col);
                        settings::set_str("xcm_dot_shape_col",       &self.xcm_dot_shape_col);
                        settings::set_str("xcm_step_start_row",      &self.xcm_step_start_row);
                        settings::set_str("xcm_gap_t1_to_t2",        &self.xcm_gap_t1_to_t2);
                        settings::set_str("xcm_density_row_offset",  &self.xcm_density_row_offset);
                        settings::set_str("xcm_title_t2_row_offset", &self.xcm_title_t2_row_offset);
                        self.show_cell_mapping_dialog = false;
                    }
                });
            if !open {
                self.show_cell_mapping_dialog = false;
            }
        }

        // ── Templates dialog ──────────────────────────────────────────────────
        if self.show_templates_dialog {
            let mut open = true;
            egui::Window::new("Templates")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("tmpl_grid").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                        // Illustrator templates
                        ui.label(egui::RichText::new("Illustrator").strong());
                        ui.label("");
                        ui.label("");
                        ui.end_row();

                        let ai_entries: &mut [(&str, &str, &mut String)] = &mut [
                            ("Standard (14 steps)", "ai_template",          &mut self.tmpl_standard),
                            ("Extended (16 steps)", "ai_template_extended", &mut self.tmpl_extended),
                        ];
                        for (label, key, path) in ai_entries.iter_mut() {
                            ui.label(*label);
                            ui.add_sized([300.0, 20.0], egui::TextEdit::singleline(*path));
                            if ui.button("Browse...").clicked() {
                                if let Some(p) = rfd::FileDialog::new()
                                    .add_filter("Illustrator Template", &["ai"])
                                    .add_filter("All Files", &["*"])
                                    .pick_file()
                                {
                                    **path = p.to_string_lossy().to_string();
                                    settings::set_str(key, path);
                                }
                            }
                            ui.end_row();
                        }

                        ui.separator();
                        ui.separator();
                        ui.separator();
                        ui.end_row();

                        // Excel template
                        ui.label(egui::RichText::new("Excel").strong());
                        ui.label("");
                        ui.label("");
                        ui.end_row();

                        ui.label("Template (.xlsx)");
                        ui.add_sized([300.0, 20.0], egui::TextEdit::singleline(&mut self.tmpl_excel));
                        if ui.button("Browse...").clicked() {
                            if let Some(p) = rfd::FileDialog::new()
                                .add_filter("Excel Template", &["xlsx"])
                                .add_filter("All Files", &["*"])
                                .pick_file()
                            {
                                self.tmpl_excel = p.to_string_lossy().to_string();
                                settings::set_str("excel_template", &self.tmpl_excel);
                            }
                        }
                        ui.end_row();

                        ui.separator();
                        ui.separator();
                        ui.separator();
                        ui.end_row();

                        // LibreOffice path (Linux PDF backend)
                        ui.label(egui::RichText::new("LibreOffice").strong());
                        ui.label("");
                        ui.label("");
                        ui.end_row();

                        ui.label("Executable path");
                        ui.add_sized([300.0, 20.0], egui::TextEdit::singleline(&mut self.tmpl_libreoffice));
                        if ui.button("Browse...").clicked() {
                            if let Some(p) = rfd::FileDialog::new()
                                .add_filter("All Files", &["*"])
                                .pick_file()
                            {
                                self.tmpl_libreoffice = p.to_string_lossy().to_string();
                                settings::set_str("libreoffice_path", &self.tmpl_libreoffice);
                            }
                        }
                        ui.end_row();
                    });
                    ui.add_space(4.0);
                    if ui.button("Save & Close").clicked() {
                        settings::set_str("ai_template",          &self.tmpl_standard);
                        settings::set_str("ai_template_extended", &self.tmpl_extended);
                        settings::set_str("excel_template",       &self.tmpl_excel);
                        settings::set_str("libreoffice_path",     &self.tmpl_libreoffice);
                        self.show_templates_dialog = false;
                    }
                });
            if !open {
                self.show_templates_dialog = false;
            }
        }

        // ── Dropdown Options dialog ───────────────────────────────────────────
        if self.show_dropdowns_dialog {
            let mut open = true;
            egui::Window::new("Dropdown Options")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Customise the options shown in each dropdown.");
                    ui.add_space(6.0);

                    // Helper closure: render one editable list section.
                    // Returns true if anything changed.
                    let render_list = |ui: &mut egui::Ui,
                                          heading: &str,
                                          items: &mut Vec<String>,
                                          id: &str|
                     -> bool {
                        let mut list_changed = false;
                        ui.strong(heading);
                        let mut remove: Option<usize> = None;
                        let n = items.len();
                        for i in 0..n {
                            ui.horizontal(|ui| {
                                if ui
                                    .add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut items[i]).id(egui::Id::new(format!("{id}_{i}"))))
                                    .changed()
                                {
                                    list_changed = true;
                                }
                                if n > 1 && ui.small_button("x").clicked() {
                                    remove = Some(i);
                                }
                            });
                        }
                        if let Some(idx) = remove {
                            items.remove(idx);
                            list_changed = true;
                        }
                        if ui.button("+ Add").clicked() {
                            items.push(String::new());
                            list_changed = true;
                        }
                        list_changed
                    };

                    render_list(ui, "Print Types", &mut self.dd_print_types, "dd_pt");
                    ui.add_space(4.0);
                    render_list(ui, "Finishes", &mut self.dd_finishes, "dd_fin");
                    ui.add_space(4.0);
                    render_list(ui, "Dot Shape Types", &mut self.dd_dot_shape_types, "dd_ds");

                    ui.add_space(8.0);
                    if ui.button("Save & Close").clicked() {
                        settings::set_string_vec("dropdown_print_types",     &self.dd_print_types);
                        settings::set_string_vec("dropdown_finishes",        &self.dd_finishes);
                        settings::set_string_vec("dropdown_dot_shape_types", &self.dd_dot_shape_types);
                        self.show_dropdowns_dialog = false;
                    }
                });
            if !open {
                self.show_dropdowns_dialog = false;
            }
        }
    }
}
