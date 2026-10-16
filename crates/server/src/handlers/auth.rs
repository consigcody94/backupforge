use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

pub async fn login(
    State(_state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // Authentication is not implemented. Refuse rather than hand out a token
    // that anyone could obtain with any password.
    let _ = req;
    Err(StatusCode::NOT_IMPLEMENTED)
}

pub async fn register(
    State(_state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<StatusCode, StatusCode> {
    let _ = req;
    Err(StatusCode::NOT_IMPLEMENTED)
}
