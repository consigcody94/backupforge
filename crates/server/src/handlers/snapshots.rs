use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use backupforge_common::types::Snapshot;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct SnapshotsListResponse {
    snapshots: Vec<Snapshot>,
}

pub async fn list_snapshots(
    State(_state): State<AppState>,
) -> Result<Json<SnapshotsListResponse>, StatusCode> {
    Ok(Json(SnapshotsListResponse { snapshots: vec![] }))
}

pub async fn get_snapshot(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Snapshot>, StatusCode> {
    Err(StatusCode::NOT_FOUND)
}

pub async fn delete_snapshot(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}
