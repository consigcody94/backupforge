use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use backupforge_common::types::Tenant;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct TenantsListResponse {
    tenants: Vec<Tenant>,
}

pub async fn list_tenants(
    State(_state): State<AppState>,
) -> Result<Json<TenantsListResponse>, StatusCode> {
    Ok(Json(TenantsListResponse { tenants: vec![] }))
}

pub async fn create_tenant(
    State(_state): State<AppState>,
    Json(tenant): Json<Tenant>,
) -> Result<Json<Tenant>, StatusCode> {
    tracing::info!("Creating tenant: {}", tenant.name);
    Ok(Json(tenant))
}

pub async fn get_tenant(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Tenant>, StatusCode> {
    Err(StatusCode::NOT_FOUND)
}
