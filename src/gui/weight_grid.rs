//! WeightGrid — data entry grid for one weight table.
//!
//! Layout:
//!        C       M       Y       K
//! D   [2.11]  [1.80]  [1.66]  [1.79]
//! 100 [100 ]  [100 ]  [100 ]  [100 ]   <- locked
//!  95 [    ]  [    ]  [    ]  [    ]
//!  ...
//!   1 [    ]  [    ]  [    ]  [    ]

use egui::Ui;

use crate::core::models::{WeightData, COLOUR_NAMES};

/// Unique ID for each cell in the grid, used for focus tracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CellId {
    row: usize, // 0 = density, 1..N = steps
    col: usize, // 0..3 = C/M/Y/K
}

/// State for the weight grid's text fields.
pub struct WeightGridState {
    /// [row][col] string values — row 0 = density, rows 1..N = steps
    pub cells: Vec<[String; 4]>,
    pub step_labels: Vec<String>,
    /// Whether the last cell was completed (signals auto-advance to next weight)
    pub completed: bool,
    /// Settle timer: which cell was last edited, and when
    settle_cell: Option<CellId>,
    settle_time: Option<f64>,
    /// Which cell should receive focus next frame
    focus_request: Option<CellId>,
    /// When the settle timer fires and auto-advances, the X-Rite Tab that arrives
    /// a frame later should be swallowed so it doesn't cause a second advance.
    advance_ignore_next_tab: bool,
}

impl WeightGridState {
    pub fn new(step_labels: &[String]) -> Self {
        let num_steps = step_labels.len();
        let mut cells = Vec::with_capacity(num_steps + 1);
        // Density row
        cells.push([String::new(), String::new(), String::new(), String::new()]);
        // Step rows
        for (i, _) in step_labels.iter().enumerate() {
            if i == 0 {
                // 100% row — locked
                cells.push(["100".into(), "100".into(), "100".into(), "100".into()]);
            } else {
                cells.push([String::new(), String::new(), String::new(), String::new()]);
            }
        }
        Self {
            cells,
            step_labels: step_labels.to_vec(),
            completed: false,
            settle_cell: None,
            settle_time: None,
            focus_request: None,
            advance_ignore_next_tab: false,
        }
    }

    pub fn from_weight_data(data: &WeightData, step_labels: &[String]) -> Self {
        let num_steps = step_labels.len();
        let mut cells = Vec::with_capacity(num_steps + 1);

        // Density row
        cells.push(data.density.map(|v| fmt(v)));

        // Step rows
        for i in 0..num_steps {
            if i == 0 {
                cells.push(["100".into(), "100".into(), "100".into(), "100".into()]);
            } else if i < data.steps.len() {
                cells.push(data.steps[i].map(|v| fmt(v)));
            } else {
                cells.push([String::new(), String::new(), String::new(), String::new()]);
            }
        }

        Self {
            cells,
            step_labels: step_labels.to_vec(),
            completed: false,
            settle_cell: None,
            settle_time: None,
            focus_request: None,
            advance_ignore_next_tab: false,
        }
    }

    pub fn to_weight_data(&self, label: &str) -> WeightData {
        let density = [
            parse_f64(&self.cells[0][0]),
            parse_f64(&self.cells[0][1]),
            parse_f64(&self.cells[0][2]),
            parse_f64(&self.cells[0][3]),
        ];

        let steps: Vec<[f64; 4]> = self.cells[1..]
            .iter()
            .map(|row| {
                [
                    parse_f64(&row[0]),
                    parse_f64(&row[1]),
                    parse_f64(&row[2]),
                    parse_f64(&row[3]),
                ]
            })
            .collect();

        WeightData {
            label: label.to_string(),
            density,
            steps,
        }
    }

    /// Column-major Tab order: density-C, step2-C, step3-C, ..., density-M, step2-M, ...
    /// Row 1 (100% step) is skipped.
    fn flat_order(&self) -> Vec<CellId> {
        let num_rows = self.cells.len();
        let mut order = Vec::new();
        for col in 0..4 {
            for row in 0..num_rows {
                if row == 1 {
                    continue; // skip locked 100% row
                }
                order.push(CellId { row, col });
            }
        }
        order
    }

    fn next_cell(&self, current: CellId) -> Option<CellId> {
        let order = self.flat_order();
        if let Some(pos) = order.iter().position(|c| *c == current) {
            if pos + 1 < order.len() {
                Some(order[pos + 1])
            } else {
                None // last cell
            }
        } else {
            None
        }
    }

    pub fn focus_first(&mut self) {
        self.focus_request = Some(CellId { row: 0, col: 0 });
    }

    /// Clear all entry fields: zero density row and step rows (re-lock 100% row).
    pub fn clear(&mut self) {
        for row in &mut self.cells {
            *row = [String::new(), String::new(), String::new(), String::new()];
        }
        // Re-lock 100% row (index 1)
        if self.cells.len() > 1 {
            self.cells[1] = ["100".into(), "100".into(), "100".into(), "100".into()];
        }
        self.settle_cell = None;
        self.settle_time = None;
        self.focus_request = None;
        self.completed = false;
        self.advance_ignore_next_tab = false;
    }
}

fn fmt(v: f64) -> String {
    if v == 0.0 {
        String::new()
    } else {
        v.to_string()
    }
}

fn parse_f64(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

/// Validate that a string is a valid partial numeric input.
fn is_valid_numeric(s: &str) -> bool {
    if s.is_empty() || s == "-" {
        return true;
    }
    if s.parse::<f64>().is_ok() {
        return true;
    }
    // Allow partial float like "2."
    if s.matches('.').count() == 1 {
        let stripped = s.replace('.', "").replace('-', "");
        return stripped.chars().all(|c| c.is_ascii_digit());
    }
    false
}

/// Draw the weight grid UI. Returns true if any value changed.
pub fn show_weight_grid(
    ui: &mut Ui,
    state: &mut WeightGridState,
    id_salt: &str,
    time: f64,
) -> bool {
    let mut changed = false;
    state.completed = false;

    let num_rows = state.cells.len();
    let flat_order = state.flat_order();
    let last_cell = flat_order.last().copied();

    egui::Grid::new(format!("weight_grid_{id_salt}"))
        .num_columns(5)
        .spacing([4.0, 2.0])
        .show(ui, |ui| {
            // Header row
            ui.label(""); // row label column
            for name in COLOUR_NAMES {
                ui.add_sized([60.0, 20.0], egui::Label::new(egui::RichText::new(*name).strong()));
            }
            ui.end_row();

            for row in 0..num_rows {
                // Row label
                let row_label = if row == 0 {
                    "D".to_string()
                } else if row - 1 < state.step_labels.len() {
                    state.step_labels[row - 1].clone()
                } else {
                    String::new()
                };
                ui.label(&row_label);

                let is_locked = row == 1; // 100% row

                for col in 0..4 {
                    let cell_id = CellId { row, col };
                    let id = egui::Id::new(format!("{id_salt}_cell_{row}_{col}"));

                    if is_locked {
                        // Show locked 100% value — same size as editable cells for uniform column width
                        ui.add_sized([60.0, 20.0], egui::Label::new("100"));
                    } else {
                        let old_val = state.cells[row][col].clone();

                        let response = ui.add_sized(
                            [60.0, 20.0],
                            egui::TextEdit::singleline(&mut state.cells[row][col])
                                .id(id)
                                .horizontal_align(egui::Align::Center),
                        );

                        // Apply queued focus request
                        if state.focus_request == Some(cell_id) {
                            response.request_focus();
                            state.focus_request = None;
                        }

                        // Validate: revert if invalid
                        if !is_valid_numeric(&state.cells[row][col]) {
                            state.cells[row][col] = old_val.clone();
                        }

                        if state.cells[row][col] != old_val {
                            changed = true;

                            // Start settle timer
                            if response.has_focus() && !state.cells[row][col].is_empty() {
                                state.settle_cell = Some(cell_id);
                                state.settle_time = Some(time);
                            }
                        }

                        // Check settle timer — auto-advance after 300ms
                        if response.has_focus() {
                            if let (Some(sc), Some(st)) = (state.settle_cell, state.settle_time) {
                                if sc == cell_id
                                    && time - st >= 0.3
                                    && !state.cells[row][col].is_empty()
                                {
                                    state.settle_cell = None;
                                    state.settle_time = None;
                                    // Tell Tab/Enter handler to swallow the X-Rite's
                                    // trailing Tab so it doesn't cause a second advance.
                                    state.advance_ignore_next_tab = true;

                                    if Some(cell_id) == last_cell {
                                        state.completed = true;
                                    } else if let Some(next) = state.next_cell(cell_id) {
                                        state.focus_request = Some(next);
                                    }
                                }
                            }
                        }

                        // Tab / Enter — column-major advance.
                        // lost_focus() fires when egui's built-in Tab moves focus away;
                        // we then override with our column-major focus_request next frame.
                        // advance_ignore_next_tab swallows the Tab that trails a settle-timer
                        // advance so it doesn't cause a second jump.
                        if response.lost_focus()
                            && ui.input(|i| {
                                i.key_pressed(egui::Key::Tab)
                                    || i.key_pressed(egui::Key::Enter)
                            })
                        {
                            if state.advance_ignore_next_tab {
                                state.advance_ignore_next_tab = false;
                            } else if Some(cell_id) == last_cell {
                                state.completed = true;
                            } else if let Some(next) = state.next_cell(cell_id) {
                                state.focus_request = Some(next);
                            }
                        }

                        // Highlight focused cell
                        if response.has_focus() {
                            let rect = response.rect;
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 50)),
                            );
                        }
                    }
                }
                ui.end_row();
            }
        });

    // Request repaint if settle timer is active (need continuous updates)
    if state.settle_cell.is_some() {
        ui.ctx().request_repaint();
    }

    changed
}
