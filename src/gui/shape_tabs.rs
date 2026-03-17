//! ShapeNotebook — outer tabs for shapes, inner tabs for weights per shape.

use crate::core::models::ShapeData;
use crate::gui::weight_grid::{self, WeightGridState};

/// State for one shape tab: contains weight sub-tabs.
pub struct ShapeTabState {
    pub name: String,
    pub weight_grids: Vec<WeightGridState>,
    pub weight_labels: Vec<String>,
    pub selected_weight: usize,
}

impl ShapeTabState {
    pub fn new(name: &str, weight_labels: &[String], step_labels: &[String]) -> Self {
        let weight_grids = weight_labels
            .iter()
            .map(|_| WeightGridState::new(step_labels))
            .collect();
        Self {
            name: name.to_string(),
            weight_grids,
            weight_labels: weight_labels.to_vec(),
            selected_weight: 0,
        }
    }

    pub fn from_shape_data(
        shape: &ShapeData,
        weight_labels: &[String],
        step_labels: &[String],
    ) -> Self {
        let weight_grids = weight_labels
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if i < shape.weights.len() {
                    WeightGridState::from_weight_data(&shape.weights[i], step_labels)
                } else {
                    WeightGridState::new(step_labels)
                }
            })
            .collect();
        Self {
            name: shape.name.clone(),
            weight_grids,
            weight_labels: weight_labels.to_vec(),
            selected_weight: 0,
        }
    }

    pub fn to_shape_data(&self) -> ShapeData {
        let weights = self
            .weight_grids
            .iter()
            .enumerate()
            .map(|(i, grid)| {
                let label = self.weight_labels.get(i).cloned().unwrap_or_default();
                grid.to_weight_data(&label)
            })
            .collect();
        ShapeData {
            name: self.name.clone(),
            weights,
        }
    }

    /// Rebuild weight grids when labels or step count changes.
    pub fn update_weights(&mut self, weight_labels: &[String], step_labels: &[String]) {
        // Collect current data
        let current_data = self.to_shape_data();

        self.weight_labels = weight_labels.to_vec();
        self.weight_grids = weight_labels
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if i < current_data.weights.len() {
                    WeightGridState::from_weight_data(&current_data.weights[i], step_labels)
                } else {
                    WeightGridState::new(step_labels)
                }
            })
            .collect();

        if self.selected_weight >= self.weight_grids.len() {
            self.selected_weight = 0;
        }
    }
}

/// State for the entire shape notebook.
pub struct ShapeNotebookState {
    pub tabs: Vec<ShapeTabState>,
    pub selected_shape: usize,
    pub weight_labels: Vec<String>,
    pub step_labels: Vec<String>,
}

impl ShapeNotebookState {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            selected_shape: 0,
            weight_labels: Vec::new(),
            step_labels: Vec::new(),
        }
    }

    pub fn populate(
        &mut self,
        shapes: &[ShapeData],
        weight_labels: &[String],
        step_labels: &[String],
    ) {
        self.weight_labels = weight_labels.to_vec();
        self.step_labels = step_labels.to_vec();
        self.tabs = shapes
            .iter()
            .map(|s| ShapeTabState::from_shape_data(s, weight_labels, step_labels))
            .collect();
        self.selected_shape = 0;
    }

    pub fn add_shape(&mut self, name: &str) {
        let tab = ShapeTabState::new(name, &self.weight_labels, &self.step_labels);
        self.tabs.push(tab);
        self.selected_shape = self.tabs.len() - 1;
    }

    pub fn remove_shape(&mut self, index: usize) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        self.tabs.remove(index);
        if self.selected_shape >= self.tabs.len() {
            self.selected_shape = self.tabs.len() - 1;
        }
        true
    }

    pub fn rename_shape(&mut self, index: usize, new_name: &str) {
        if index < self.tabs.len() {
            self.tabs[index].name = new_name.to_string();
        }
    }

    pub fn update_weight_labels(&mut self, labels: &[String]) {
        self.weight_labels = labels.to_vec();
        for tab in &mut self.tabs {
            tab.update_weights(labels, &self.step_labels);
        }
    }

    pub fn update_step_labels(&mut self, labels: &[String]) {
        self.step_labels = labels.to_vec();
        for tab in &mut self.tabs {
            tab.update_weights(&self.weight_labels, labels);
        }
    }

    pub fn get_all_shapes(&self) -> Vec<ShapeData> {
        self.tabs.iter().map(|t| t.to_shape_data()).collect()
    }

    /// Clear the currently visible weight grid.
    pub fn clear_current_weight(&mut self) {
        let si = self.selected_shape;
        if let Some(tab) = self.tabs.get_mut(si) {
            let wi = tab.selected_weight;
            if let Some(grid) = tab.weight_grids.get_mut(wi) {
                grid.clear();
            }
        }
    }

    /// Clear all weight grids across all shapes.
    pub fn clear_all_weights(&mut self) {
        for tab in &mut self.tabs {
            for grid in &mut tab.weight_grids {
                grid.clear();
            }
        }
    }
}

/// Draw the shape notebook UI. Returns true if any data changed.
pub fn show_shape_notebook(
    ui: &mut egui::Ui,
    state: &mut ShapeNotebookState,
    time: f64,
) -> bool {
    let mut changed = false;

    if state.tabs.is_empty() {
        ui.label("No shapes. Click '+ Add Shape' to begin.");
        return false;
    }

    // Shape tabs (outer)
    ui.horizontal(|ui| {
        for (i, tab) in state.tabs.iter().enumerate() {
            let selected = i == state.selected_shape;
            if ui.selectable_label(selected, &tab.name).clicked() {
                state.selected_shape = i;
            }
        }
    });

    ui.separator();

    let shape_idx = state.selected_shape;
    if shape_idx >= state.tabs.len() {
        return false;
    }

    let tab = &mut state.tabs[shape_idx];

    // Weight sub-tabs (inner)
    ui.horizontal(|ui| {
        for (i, label) in tab.weight_labels.iter().enumerate() {
            let selected = i == tab.selected_weight;
            if ui.selectable_label(selected, label).clicked() {
                tab.selected_weight = i;
            }
        }
    });

    ui.separator();

    let weight_idx = tab.selected_weight;
    if weight_idx >= tab.weight_grids.len() {
        return false;
    }

    // Show the weight grid
    let grid_id = format!("shape_{shape_idx}_weight_{weight_idx}");
    let grid_changed = egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            weight_grid::show_weight_grid(
                ui,
                &mut tab.weight_grids[weight_idx],
                &grid_id,
                time,
            )
        })
        .inner;

    if grid_changed {
        changed = true;
    }

    // Auto-advance to next weight tab when grid completes
    if tab.weight_grids[weight_idx].completed {
        tab.weight_grids[weight_idx].completed = false;
        if weight_idx + 1 < tab.weight_grids.len() {
            tab.selected_weight = weight_idx + 1;
            tab.weight_grids[weight_idx + 1].focus_first();
        } else {
            // Last weight — wrap to first
            tab.weight_grids[weight_idx].focus_first();
        }
    }

    changed
}
