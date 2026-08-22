use axum::{routing::get, Router};
use crate::handlers::{health_check};
use crate::handlers::{get_upload_url};

pub fn create_routes() -> Router {
    Router::new()
        .route("/health",
               get(health_check))
        .route("/upload_url",
               get(get_upload_url))
}

