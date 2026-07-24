# xrite-export — Operations

## Build deps

None beyond a standard Rust toolchain. The desktop egui/eframe GUI (which needed
GTK/libxcb) was removed — `src/gui/` had drifted out of sync with the current
data model and wasn't wired into `main.rs` anyway. The web build (`--features web`)
is the only functional mode.

## Dev mode

```bash
cargo run --features web -- --web        # web server on :8181
cargo test                                # unit tests
cargo test round_trip                     # single test by name substring
```

`cargo run` without `--web` prints "Desktop mode is not yet available in this build."

## Production build + deploy (Linux web)

```bash
cargo build --release --features web
sudo systemctl restart ink-density-tool
sudo systemctl status ink-density-tool
journalctl -u ink-density-tool -f        # tail logs
```

The service unit (`deploy/ink-density-tool.service`) is symlinked from `/etc/systemd/system/ink-density-tool.service`. Editing it in the repo + `sudo systemctl daemon-reload` is enough.

**Important:** the running binary is loaded into memory. Recompiling without a restart will NOT update the live server.

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
| Web server returns old data after recompile | systemd unit not restarted | `sudo systemctl restart ink-density-tool` — recompiling alone doesn't update the running process |
| Report always comes out landscape/portrait unexpectedly | `report_orientation` setting | Toggle it in the web Settings modal (Report Layout section) — it's read fresh from disk on every export request |
