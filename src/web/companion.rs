//! Companion server — localhost:7432 (Windows)
//!
//! Bridges the browser to Illustrator on the Windows client.
//! All responses include PNA CORS headers so Chrome/Firefox allow
//! cross-origin requests from the HTTPS-tunnelled main app.
//!
//! Routes:
//!   GET     /health      → 200 OK  (probed by browser on page load)
//!   POST    /export/pdf  → body = JobConfig JSON → stream PDF bytes
//!   GET     /settings    → Illustrator/template settings JSON
//!   POST    /settings    → patch Illustrator/template settings
//!   OPTIONS *            → PNA preflight

use std::collections::HashMap;

use axum::{
    extract::Query,
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use tokio::net::TcpListener;

use crate::core::models::JobConfig;
use crate::settings;

pub fn run() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(serve());
}

async fn serve() {
    let app = Router::new()
        .route("/health",      get(health).options(preflight))
        .route("/export/pdf",  post(export_pdf).options(preflight))
        .route("/settings",    get(get_settings).post(patch_settings).options(preflight))
        .route("/browse/file", get(browse_file).options(preflight));

    let listener = TcpListener::bind("127.0.0.1:7432")
        .await
        .expect("Failed to bind localhost:7432");
    println!("Ink Density Tool companion running on http://127.0.0.1:7432");
    axum::serve(listener, app).await.expect("Companion error");
}

// ── PNA helpers ───────────────────────────────────────────────────────────────

fn pna(status: StatusCode, content_type: &str, body: Vec<u8>) -> Response {
    Response::builder()
        .status(status)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Private-Network", "true")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .header("Content-Type", content_type)
        .body(body.into())
        .unwrap()
}

fn pna_err(msg: String) -> Response {
    pna(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", msg.into_bytes())
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn preflight() -> Response {
    pna(StatusCode::OK, "text/plain", b"OK".to_vec())
}

async fn health() -> Response {
    pna(StatusCode::OK, "text/plain", b"OK".to_vec())
}

async fn export_pdf(Json(job): Json<JobConfig>) -> Response {
    let tmp = match tempfile::NamedTempFile::new() {
        Ok(f) => f,
        Err(e) => return pna_err(e.to_string()),
    };

    match crate::export::illustrator::export_pdf(&job, tmp.path()) {
        Ok(()) => {
            let bytes = match std::fs::read(tmp.path()) {
                Ok(b) => b,
                Err(e) => return pna_err(e.to_string()),
            };
            let filename = if job.job_number.is_empty() {
                "export.pdf".to_string()
            } else {
                format!("{}.pdf", job.job_number)
            };
            Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Private-Network", "true")
                .header("Content-Type", "application/pdf")
                .header("Content-Disposition", format!("attachment; filename=\"{filename}\""))
                .body(bytes.into())
                .unwrap()
        }
        Err(e) => pna_err(e.to_string()),
    }
}

/// Open a native file-picker dialog on the Windows PC and return the selected path.
/// Query param: `filter=ai` for .ai files, `filter=xlsx` for .xlsx, omit for all files.
/// Returns 200 `{"path": "..."}` on selection, 204 if the user cancelled.
async fn browse_file(Query(params): Query<HashMap<String, String>>) -> Response {
    let filter = params.get("filter").cloned().unwrap_or_default();
    let result = tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();
        dialog = match filter.as_str() {
            "ai"   => dialog.add_filter("Illustrator Template", &["ai"]).add_filter("All Files", &["*"]),
            "xlsx" => dialog.add_filter("Excel Template", &["xlsx"]).add_filter("All Files", &["*"]),
            _      => dialog.add_filter("All Files", &["*"]),
        };
        dialog.pick_file()
    })
    .await;

    match result {
        Ok(Some(path)) => {
            let body = serde_json::json!({ "path": path.to_string_lossy() }).to_string();
            pna(StatusCode::OK, "application/json", body.into_bytes())
        }
        Ok(None) => pna(StatusCode::NO_CONTENT, "text/plain", b"Cancelled".to_vec()),
        Err(e) => pna_err(e.to_string()),
    }
}

const COMPANION_KEYS: &[&str] = &["illustrator_path", "ai_template", "ai_template_extended"];

async fn get_settings() -> Response {
    let s = settings::load();
    let body = serde_json::json!({
        "illustrator_path":    s.illustrator_path,
        "ai_template":         s.ai_template,
        "ai_template_extended": s.ai_template_extended,
    });
    pna(StatusCode::OK, "application/json", serde_json::to_vec(&body).unwrap_or_default())
}

async fn patch_settings(Json(patch): Json<serde_json::Value>) -> Response {
    if let Some(obj) = patch.as_object() {
        for (key, value) in obj {
            if COMPANION_KEYS.contains(&key.as_str()) {
                settings::patch(key, value.clone());
            }
        }
    }
    pna(StatusCode::OK, "text/plain", b"OK".to_vec())
}
