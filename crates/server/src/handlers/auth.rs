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
    // Placeholder - would verify credentials against database
    tracing::info!("Login attempt for user: {}", req.username);

    // Generate JWT token (placeholder)
    Ok(Json(LoginResponse {
        token: "placeholder_token".to_string(),
    }))
}

pub async fn register(
    State(_state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<StatusCode, StatusCode> {
    // Placeholder - would create user in database
    tracing::info!("Registration attempt for user: {}", req.username);

    Ok(StatusCode::CREATED)
}
