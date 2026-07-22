# xrite-export — Operations

## Linux build deps (one-time)

```bash
sudo apt-get install \
  libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

These are required for egui/eframe. The web build doesn't strictly need GTK, but the same crate tree is used either way.

## Dev mode

```bash
cargo run                                # desktop egui app
cargo run --features web -- --web        # web server on :8181
cargo run --features web -- --companion  # Windows companion on :7432
cargo test                               # unit tests
cargo test round_trip                    # single test by name substring
```

## Production build + deploy (Linux web)

```bash
cargo build --release --features web
sudo systemctl restart ink-density-tool
sudo systemctl status ink-density-tool
journalctl -u ink-density-tool -f        # tail logs
```

The service unit (`deploy/ink-density-tool.service`) is symlinked from `/etc/systemd/system/ink-density-tool.service`. Editing it in the repo + `sudo systemctl daemon-reload` is enough.

**Important:** the running binary is loaded into memory. Recompiling without a restart will NOT update the live server.

## Cross-compile for Windows (desktop EXE)

```bash
cargo build --release --target x86_64-pc-windows-gnu
# → target/x86_64-pc-windows-gnu/release/ink-density-tool.exe
```

For the companion EXE, add `--features web`:

```bash
cargo build --release --target x86_64-pc-windows-gnu --features web
# → same path; runs as `ink-density-tool.exe --companion` on Windows
```

You'll need `mingw-w64` and the rust target installed:

```bash
sudo apt-get install mingw-w64
rustup target add x86_64-pc-windows-gnu
```

## Domain migration in progress

The tool is currently reachable at both:
- `xrite-export.elphiene.com` (legacy — being phased out)
- `xrite.rgb-b.com` (new canonical)

Both route to the same `:8181` service via the host's cloudflared tunnel. Once `xrite.rgb-b.com` is verified working end-to-end:

1. Remove the `xrite-export.elphiene.com` ingress entry from `~/.cloudflared/config.yml`
2. Remove the DNS record for `xrite-export.elphiene.com` in El's Cloudflare account
3. `sudo systemctl restart cloudflared`

## Healthcheck

```bash
systemctl is-active ink-density-tool.service
curl -s http://localhost:8181/api/version
curl -s https://xrite.rgb-b.com/api/version
```

`/api/version` returns `{"build_ts": <unix>}` — the embedded `BUILD_TIMESTAMP` from `build.rs`. If the value doesn't change after a deploy, the service didn't pick up the new binary.

`check-projects` covers `ink-density-tool.service` automatically.

## Settings + session storage

Settings live at `~/InkDensityTool/settings.json` (resolved via `dirs` crate — XDG-compliant on Linux, AppData on Windows). Session files are user-saved JSON anywhere on disk.

To reset settings to defaults: delete `~/InkDensityTool/settings.json` and restart the app.

## Common issues

| Symptom | Cause | Fix |
|---|---|---|
| Browser can't generate PDFs on macOS/Linux | Companion EXE not running on a Windows machine | Run the companion EXE on Windows; the frontend probes `localhost:7432/health` on load |
| Excel export panics | Temp file missing `.xlsx` suffix | Use `tempfile::Builder::new().suffix(".xlsx").tempfile()` |
| Excel formulas show as plaintext | Code called `set_formula()` then re-saved | Don't. Formulas are baked into `assets/template_extended.xlsx`. `fix_second_table_formulas` and `clone_first_pair` are dead code — do not invoke. |
| Web server returns old data after recompile | systemd unit not restarted | `sudo systemctl restart ink-density-tool` — recompiling alone doesn't update the running process |
| PNA CORS errors when probing localhost from HTTPS page | Browser blocks Private Network Access by default | Companion sends `Access-Control-Allow-Private-Network: true` — verify that header isn't being stripped by a proxy |
