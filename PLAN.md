# Ink Density Tool — Rebuild Plan

## Overview

A tool for print professionals to record CMYK + spot ink density readings from an X-Rite eXact spectrodensitometer, then export data to Excel and print-ready PDF reports.

**Deployed at:** `xrite.rgb-b.com` (Cloudflare Tunnel → axum :8181)
**Repo:** `rgb-b/xrite-export`
**Stack:** Rust / axum (web) / egui (desktop) / umya-spreadsheet / single-file HTML frontend

---

## Key Concepts

### No hardcoded scan modes
Everything is driven by **presets**. A preset defines which fields are visible, the grid layout, the report layout, and the default step preset. Users can create presets named anything ("Gradation Standard", "Sample Scan", "Quick Check", etc.).

### Inks drive data columns
The user selects which inks are present per job: C, M, Y, K, White, and any number of named spot colours (e.g. "PMS 485"). The data grid renders one column per ink. Spot colours are excluded from the deviation average.

### Step presets + interpolation
Step lists are named, saveable presets. For any step value, the target is resolved via **linear interpolation** between the 16 known dot gain anchor points. Steps outside the 0.4–100 range have no target (deviation cell left blank).

### Dot gain target anchors (fixed)
| Step | Target | Step | Target |
|------|--------|------|--------|
| 100  | 100    | 30   | 51     |
| 95   | 99     | 20   | 37     |
| 90   | 97     | 10   | 22     |
| 80   | 93     | 5    | 13     |
| 70   | 88     | 3    | 9      |
| 60   | 81     | 1    | 3      |
| 50   | 72     | 0.8  | 2      |
| 40   | 62     | 0.4  | 1      |

Deviation = AVERAGE(CMYK columns only) − interpolated target. Spot inks excluded from average.

### Comparison reports
Load two separate saved sessions → render side by side or stacked → browser print to PDF.

---

## Data Model

```rust
enum InkKind { Cyan, Magenta, Yellow, Black, White, Spot }
struct Ink { kind: InkKind, name: String }

struct WeightData {
    lpi: String,
    density: Vec<f64>,       // one per ink (dynamic, not fixed [4])
    steps: Vec<Vec<f64>>,    // [step_idx][ink_idx]
}

struct ShapeData {
    dot_type: String,        // "CRS", "HD", "ESXR", …
    dot_number: String,      // "501", "16", …
    weights: Vec<WeightData>,
}

struct JobConfig {
    preset_name: String,

    // Metadata — preset controls which are visible/used
    job_name: String,
    job_number: String,
    customer: String,
    plate_tech: String,      // "CRS" (Crystal) | "QUA" (Quartz) | ""
    press_system: String,    // "XPS" | "ITP" | ""
    esxr_number: String,     // optional, only shown when relevant
    print_type: String,      // "RP" | "SP" | "CBW SP" | ""
    date: String,
    set_number: String,

    inks: Vec<Ink>,
    step_labels: Vec<String>,
    shapes: Vec<ShapeData>,
}

// Auto-assembled heading: "{customer} — {plate_tech} {press_system} {esxr_number} — {print_type}"

struct FieldVisibility {
    job_name: bool, job_number: bool, customer: bool,
    plate_tech: bool, press_system: bool, esxr_number: bool,
    print_type: bool, date: bool, set_number: bool,
    inks: bool, lpis: bool, steps: bool,
}

enum GridLayout {
    Tabbed,  // shape tabs → LPI tabs → grid (multi-shape, multi-LPI jobs)
    Flat,    // single grid, no tab hierarchy (quick scans)
}

enum ReportLayout { Single, DualComparison }

struct JobPreset {
    name: String,
    fields: FieldVisibility,
    grid_layout: GridLayout,
    report_layout: ReportLayout,
    default_step_preset: Option<String>,
}

struct StepPreset {
    name: String,
    steps: Vec<String>,   // ["100","75","50","25","10","2"]
}

struct Settings {
    job_presets: Vec<JobPreset>,
    step_presets: Vec<StepPreset>,  // includes "Standard 14" and "Extended 16" as built-ins
    last_session_path: String,
    // companion (optional, for Illustrator users)
    illustrator_path: String,
    ai_template: String,
    ai_template_extended: String,
}
```

---

## Module Layout

```
src/
├── main.rs                  # entry: desktop | --web | --companion
├── settings.rs              # Settings, JobPreset, StepPreset — JSON persistence
├── core/
│   ├── models.rs            # JobConfig, ShapeData, WeightData, Ink, InkKind
│   ├── session.rs           # save_session / load_session (forward-compat)
│   └── targets.rs           # DOT_GAIN_TARGETS, interpolate_target(step) -> Option<f64>
├── export/
│   ├── excel.rs             # generate from scratch (no templates)
│   ├── report.rs            # HTML report generator (single + dual-comparison)
│   ├── svg.rs               # SVG export (polished)
│   ├── placeholders.rs      # kept for companion
│   ├── illustrator.rs       # kept for companion
│   └── pdf_merge.rs         # kept for companion
├── gui/                     # egui desktop (updated to new model)
│   ├── app.rs
│   ├── job_config.rs
│   ├── shape_tabs.rs
│   └── weight_grid.rs
└── web/
    ├── server.rs            # routes + /api/export/report
    └── companion.rs         # unchanged

assets/
    index.html               # rebuilt frontend (single file)
```

---

## Web API Routes

```
GET  /                           → index.html
GET  /api/job                    → current JobConfig JSON
POST /api/job                    → replace in-memory job state
GET  /api/settings               → full settings JSON (presets included)
POST /api/settings               → patch settings
POST /api/export/excel           → body: JobConfig → stream .xlsx
GET  /api/export/report          → body/params: JobConfig(s) → print-ready HTML
POST /api/export/svg             → body: JobConfig → stream .svg
GET  /api/export/builder-script  → download build_ai_template.jsx
GET  /api/version                → { build_ts }
```

---

## UI Structure

**Toolbar:** `[Load Preset ▾]  [New Job]  [Save Session]  [Load Session]  ──  [Export Excel]  [Export PDF]  [Export SVG]  [Settings ⚙]`

**Left panel** (only fields enabled by active preset):
- Job info: name, number, customer, set #, date
- Spec: plate tech (CRS/QUA toggle), press system (XPS/ITP toggle), ESXR number, print type (RP/SP/CBW SP)
- Inks: C M Y K W checkboxes + `[+ Add spot…]` (user names it)
- Steps: step preset dropdown + inline custom editor
- Auto-heading preview (live): `Acme Corp — CRS XPS — RP`

**Center:**
- `Tabbed` layout: shape tabs → LPI tabs → grid
- `Flat` layout: single grid, no tabs

**Grid columns:** `Step | [C] [M] [Y] [K] [spot…] | Avg | Dev`
- Max Density row: editable, no target comparison
- 100% row: read-only (always 100)
- Remaining steps: editable
- Avg + Dev computed live in browser (spots excluded from avg)

---

## Export

### Excel (generated, no templates)
- Sheet structure built dynamically from active inks + steps
- One sheet pair per shape: single-LPI sheet + multi-LPI sheet
- Avg formula: `=AVERAGE(cmyk_cols_this_row)` — live Excel formula
- Deviation formula: `=avg_cell - {target}` — target baked in per row, blank if no target for that step
- Comparison layout: two job blocks on same sheet

### PDF (browser print)
- `GET /api/export/report` returns styled, print-ready HTML
- Single job or dual-comparison (two sessions POSTed together)
- Print CSS: page breaks between shapes, no UI chrome
- No Illustrator, no companion, no Windows dependency

### SVG
- Polish existing WIP into production-ready output
- One page per 3-weight chunk (matches old layout)

---

## Infrastructure

- **Single repo:** `rgb-b/xrite-export` (data-export duplication removed)
- **Domain:** `xrite.rgb-b.com` — Cloudflare Tunnel config + DNS only, no code change
- **Companion:** kept, untouched, optional for Illustrator users
- **Desktop distribution:** not a priority; web app is primary

---

## Build Order

1. `core/targets.rs` — dot gain table + interpolation
2. `core/models.rs` — new data model (Ink, dynamic WeightData, JobConfig)
3. `settings.rs` — JobPreset, StepPreset, Settings persistence
4. `core/session.rs` — forward-compat save/load
5. `web/server.rs` — updated routes
6. `assets/index.html` — rebuilt frontend
7. `export/excel.rs` — generate-from-scratch engine
8. `export/report.rs` — HTML report (single + dual)
9. `export/svg.rs` — polish
10. Companion — verify still works with new model
11. `gui/` — update egui desktop to new model
12. Visual polish on Excel + PDF (iterative, after core works)

---

## Open / Future
- Dot gain curve visualisation in Settings (show the 16-point curve + interpolated values)
- Configurable target table (if standards change)
- Comparison report UI for loading two sessions in the browser
