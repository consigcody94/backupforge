use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use backupforge_common::types::BackupJob;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct JobsListResponse {
    jobs: Vec<BackupJob>,
}

pub async fn list_jobs(
    State(_state): State<AppState>,
) -> Result<Json<JobsListResponse>, StatusCode> {
    Ok(Json(JobsListResponse { jobs: vec![] }))
}

pub async fn create_job(
    State(_state): State<AppState>,
    Json(job): Json<BackupJob>,
) -> Result<Json<BackupJob>, StatusCode> {
    tracing::info!("Creating backup job: {}", job.name);
    Ok(Json(job))
}

pub async fn get_job(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<BackupJob>, StatusCode> {
    Err(StatusCode::NOT_FOUND)
}

pub async fn delete_job(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn run_job(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Running job: {}", id);
    Ok(StatusCode::ACCEPTED)
}
