# Ink Density Tool

A web app for print professionals to record CMYK ink density readings from an X-Rite eXact
spectrodensitometer and export print-ready HTML reports and SVG.

Built with Rust + [axum](https://github.com/tokio-rs/axum).

---

## Features

- Enter CMYK density readings across 14-step or 16-step tonal scales
- Support for multiple dot shapes and LPI weights per job
- Print-ready HTML report — landscape (shapes side-by-side) or portrait (stacked), toggle in Settings
- SVG export
- Save/load sessions as JSON
- Job presets, step presets, and dot-type/LPI picker options, all configurable in Settings

---

## Build

```bash
cargo run --features web -- --web   # dev web server on :8181
cargo build --release --features web
cargo test
```

> Desktop mode (`cargo run` without `--web`) isn't available yet — see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## Project Structure

```
src/
├── main.rs                    # Entry point
├── settings.rs                # Persistent settings (%APPDATA%/InkDensityTool/)
├── core/
│   ├── models.rs              # JobConfig, ShapeData, WeightData
│   └── session.rs             # Save/load session JSON
├── export/
│   ├── report.rs              # HTML report + comparison report
│   └── svg.rs                 # SVG export
└── web/
    └── server.rs              # axum server — serves assets/index.html + /api/*
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full data model, web routes, and export flow.
