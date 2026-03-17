//! Main application window — egui/eframe based.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::core::models::{JobConfig, ShapeData, WeightData};
use crate::core::session;
use crate::gui::job_config::{self, JobConfigState};
use crate::gui::shape_tabs::{self, ShapeNotebookState};
use crate::settings;

/// Export status shared between main thread and background export threads.
#[derive(Clone, Default)]
struct ExportStatus {
    message: String,
    is_error: bool,
    busy: bool,
}

pub struct InkDensityApp {
    config_state: JobConfigState,
    shape_state: ShapeNotebookState,
    current_path: Option<PathBuf>,
    dirty: bool,
    status: String,
    export_status: Arc<Mutex<ExportStatus>>,

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
            show_add_shape_dialog: false,
            new_shape_name: String::new(),
            show_rename_dialog: false,
            rename_shape_name: String::new(),
            rename_shape_idx: 0,
            show_templates_dialog: false,
            tmpl_standard: settings::get_str("ai_template"),
            tmpl_extended: settings::get_str("ai_template_extended"),
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

        // Build weights first (while closure borrows are active), then move step_labels
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
        // Check export status from background thread
        {
            let s = self.export_status.lock().unwrap();
            if !s.message.is_empty() {
                self.status = s.message.clone();
            }
            if s.busy {
                ctx.request_repaint();
            }
        }

        // Keyboard shortcuts
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                // defer to avoid borrow issues
            }
        });

        // Top menu bar
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
                });
            });
        });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Status: {}", self.status));
            });
        });

        // Left panel — job config
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

        // Central panel — shape tabs + toolbar
        egui::CentralPanel::default().show(ctx, |ui| {
            // Toolbar
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

            // Export buttons at bottom
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

        // Add shape dialog
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

        // Rename dialog
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

        // Templates dialog
        if self.show_templates_dialog {
            let mut open = true;
            egui::Window::new("Illustrator Templates")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("tmpl_grid").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                        let entries: &mut [(&str, &str, &mut String)] = &mut [
                            ("Standard (14 steps)", "ai_template",          &mut self.tmpl_standard),
                            ("Extended (16 steps)", "ai_template_extended", &mut self.tmpl_extended),
                        ];
                        for (label, key, path) in entries.iter_mut() {
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
                    });
                    ui.add_space(4.0);
                    if ui.button("Save & Close").clicked() {
                        settings::set_str("ai_template",          &self.tmpl_standard);
                        settings::set_str("ai_template_extended", &self.tmpl_extended);
                        self.show_templates_dialog = false;
                    }
                });
            if !open {
                self.show_templates_dialog = false;
            }
        }
    }
}
