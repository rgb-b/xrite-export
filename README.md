# Ink Density Tool

A desktop GUI application for print professionals to record CMYK ink density readings from an X-Rite eXact spectrodensitometer and export the data to Adobe Illustrator-generated PDFs and Excel workbooks.

Built with Rust + [egui](https://github.com/emilk/egui).

---

## Features

- Enter CMYK density readings across 14-step or 16-step tonal scales
- Support for multiple dot shapes and LPI weights per job
- Column-major tab order to match the X-Rite eXact scan sequence
- Auto-advance between fields (300 ms settle timer)
- Export to PDF via Adobe Illustrator (Windows) or LibreOffice (Linux)
- Export to Excel using configurable cell mapping templates
- Save/load sessions as JSON
- Configurable Illustrator template paths, default labels, and cell mapping
- Tools menu to extract the Illustrator builder script from the binary

---

## Requirements

### Windows (runtime)
- Adobe Illustrator (for PDF export)
- Pre-built `.ai` template files (see [TEMPLATE_GUIDE.md](TEMPLATE_GUIDE.md))
- Excel `.xlsx` template files

### Linux (build dependencies)
```bash
sudo apt-get install libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

---

## Build

```bash
# Run in dev mode
cargo run

# Release build (Linux)
cargo build --release

# Cross-compile Windows EXE (from Linux)
cargo build --release --target x86_64-pc-windows-gnu

# Run tests
cargo test
```

The Windows EXE will be at:
```
target/x86_64-pc-windows-gnu/release/ink-density-tool.exe
```

---

## First-time Setup (Windows)

1. Run `ink-density-tool.exe`
2. Open **Settings → Templates** and point the app at your `.ai` template files and `.xlsx` template files
3. Optionally open **Settings → Dropdown Options** to configure default weight and step labels
4. Use **Tools → Export Builder Script** to extract `build_ai_template.jsx` — run this in Illustrator (**File → Scripts → Other Script**) to generate the `.ai` template files if you don't have them yet

---

## Project Structure

```
src/
├── main.rs                    # Entry point
├── settings.rs                # Persistent settings (%APPDATA%/InkDensityTool/)
├── core/
│   ├── models.rs              # JobConfig, ShapeData, WeightData
│   └── session.rs             # Save/load session JSON
├── gui/
│   ├── app.rs                 # Top-level app + menus
│   ├── job_config.rs          # Left panel: metadata + weight chips
│   ├── shape_tabs.rs          # Shape/weight tab switcher
│   └── weight_grid.rs         # Data-entry grid
└── export/
    ├── placeholders.rs        # <<PLACEHOLDER>> dict builder
    ├── illustrator.rs         # ExtendScript subprocess (Windows)
    ├── libreoffice.rs         # LibreOffice UNO bridge (Linux)
    ├── excel.rs               # Excel template fill
    └── pdf_merge.rs           # PDF merge
```

---

## Template Placeholders

See [TEMPLATE_GUIDE.md](TEMPLATE_GUIDE.md) for the full list of `<<PLACEHOLDER>>` names used in Illustrator `.ai` templates.
