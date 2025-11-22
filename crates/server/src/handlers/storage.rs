use axum::{extract::State, http::StatusCode, Json};
use backupforge_storage::StorageStats;

use crate::state::AppState;

pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<StorageStats>, StatusCode> {
    let agent_lock = state.agent.read().await;

    if let Some(ref agent) = *agent_lock {
        let stats = agent
            .storage()
            .stats()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(stats))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}
