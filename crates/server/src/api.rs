use axum::{
    routing::{get, post, delete},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::state::AppState;

/// Create the main application router
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))

        // Authentication
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/register", post(handlers::auth::register))

        // Backup jobs
        .route("/api/jobs", get(handlers::jobs::list_jobs))
        .route("/api/jobs", post(handlers::jobs::create_job))
        .route("/api/jobs/:id", get(handlers::jobs::get_job))
        .route("/api/jobs/:id", delete(handlers::jobs::delete_job))
        .route("/api/jobs/:id/run", post(handlers::jobs::run_job))

        // Snapshots
        .route("/api/snapshots", get(handlers::snapshots::list_snapshots))
        .route("/api/snapshots/:id", get(handlers::snapshots::get_snapshot))
        .route("/api/snapshots/:id", delete(handlers::snapshots::delete_snapshot))

        // Storage
        .route("/api/storage/stats", get(handlers::storage::get_stats))

        // Tenants (multi-tenancy)
        .route("/api/tenants", get(handlers::tenants::list_tenants))
        .route("/api/tenants", post(handlers::tenants::create_tenant))
        .route("/api/tenants/:id", get(handlers::tenants::get_tenant))

        // Static files for web dashboard
        .nest_service("/", tower_http::services::ServeDir::new("web-dashboard/dist"))

        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
