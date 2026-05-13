//! Main web server — axum :8181
//!
//! Routes:
//!   GET  /                             → serve embedded index.html
//!   GET  /api/job                      → current JobConfig as JSON
//!   POST /api/job                      → replace in-memory job state
//!   GET  /api/settings                 → settings JSON
//!   POST /api/settings                 → replace settings
//!   POST /api/export/excel             → body = JobConfig JSON → stream .xlsx
//!   POST /api/export/report            → body = JobConfig JSON → print-ready HTML
//!   POST /api/export/comparison        → body = [JobConfig, …] JSON → print-ready HTML
//!   POST /api/export/svg               → body = JobConfig JSON → stream .svg
//!   GET  /api/export/builder-script    → download build_ai_template.jsx
//!   GET  /api/version                  → { build_ts }

use std::sync::{Arc, Mutex};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use tokio::net::TcpListener;

use crate::core::models::JobConfig;
use crate::settings::{self, Settings};

type SharedJob = Arc<Mutex<JobConfig>>;

const INDEX_HTML: &[u8] = include_bytes!("../../assets/index.html");
const BUILDER_JSX: &[u8] = include_bytes!("../../assets/build_ai_template.jsx");
const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");

pub fn run() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(serve());
}

async fn serve() {
    let job = restore_last_session().unwrap_or_default();
    let state: SharedJob = Arc::new(Mutex::new(job));

    let app = Router::new()
        .route("/", get(index))
        .route("/api/job", get(get_job).post(set_job))
        .route("/api/settings", get(get_settings).post(put_settings))
        .route("/api/export/excel", post(export_excel))
        .route("/api/export/report", post(export_report))
        .route("/api/export/comparison", post(export_comparison))
        .route("/api/export/svg", post(export_svg_handler))
        .route("/api/export/builder-script", get(download_builder_script))
        .route("/api/version", get(get_version))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:8181")
        .await
        .expect("Failed to bind :8181");
    println!("Ink Density Tool web server running on http://0.0.0.0:8181");
    axum::serve(listener, app).await.expect("Server error");
}

fn restore_last_session() -> Option<JobConfig> {
    let path_str = settings::get_str("last_session_path");
    if path_str.is_empty() {
        return None;
    }
    crate::core::session::load_session(std::path::Path::new(&path_str)).ok()
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn index() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(INDEX_HTML.to_vec().into())
        .unwrap()
}

async fn get_job(State(state): State<SharedJob>) -> Response {
    let job = state.lock().unwrap().clone();
    json_response(&job)
}

async fn set_job(
    State(state): State<SharedJob>,
    Json(job): Json<JobConfig>,
) -> impl IntoResponse {
    *state.lock().unwrap() = job;
    StatusCode::OK
}

async fn get_settings() -> Response {
    let s = settings::load();
    json_response(&s)
}

async fn put_settings(Json(new_settings): Json<Settings>) -> impl IntoResponse {
    let _ = settings::save(&new_settings);
    StatusCode::OK
}

async fn export_excel(Json(job): Json<JobConfig>) -> Response {
    let tmp = match tempfile::Builder::new().suffix(".xlsx").tempfile() {
        Ok(f) => f,
        Err(e) => return error_response(e.to_string()),
    };

    match crate::export::excel::export_excel(&job, tmp.path()) {
        Ok(()) => {
            let bytes = match std::fs::read(tmp.path()) {
                Ok(b) => b,
                Err(e) => return error_response(e.to_string()),
            };
            let filename = if job.job_number.is_empty() {
                "export.xlsx".to_string()
            } else {
                format!("{}.xlsx", job.job_number)
            };
            Response::builder()
                .status(StatusCode::OK)
                .header(
                    "Content-Type",
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                )
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"{filename}\""),
                )
                .body(bytes.into())
                .unwrap()
        }
        Err(e) => error_response(e.to_string()),
    }
}

async fn export_report(Json(job): Json<JobConfig>) -> Response {
    let html = crate::export::report::generate_report(&job);
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html.into())
        .unwrap()
}

async fn export_comparison(Json(jobs): Json<Vec<JobConfig>>) -> Response {
    let refs: Vec<&JobConfig> = jobs.iter().collect();
    let html = crate::export::report::generate_comparison_report(&refs);
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html.into())
        .unwrap()
}

async fn export_svg_handler(Json(job): Json<JobConfig>) -> Response {
    let svg = crate::export::svg::export_svg(&job);
    let filename = if job.job_number.is_empty() {
        "export.svg".to_string()
    } else {
        format!("{}.svg", job.job_number)
    };
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/svg+xml")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        )
        .body(svg.into_bytes().into())
        .unwrap()
}

async fn get_version() -> Response {
    json_response(&serde_json::json!({ "build_ts": BUILD_TIMESTAMP }))
}

async fn download_builder_script() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/javascript")
        .header(
            "Content-Disposition",
            "attachment; filename=\"build_ai_template.jsx\"",
        )
        .body(BUILDER_JSX.to_vec().into())
        .unwrap()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn json_response<T: serde::Serialize>(data: &T) -> Response {
    let body = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(body.into())
        .unwrap()
}

fn error_response(msg: String) -> Response {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("Content-Type", "text/plain")
        .body(msg.into())
        .unwrap()
}
