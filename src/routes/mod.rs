use crate::handlers::photos::{delete_file, get_files, get_metadata, health, upload_file};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::Method,
    routing::{delete, get, put},
};
use tower_http::cors::CorsLayer;

pub fn router() -> Router {
    dotenvy::dotenv().ok();

    let allowed_origins = [std::env::var("ALLOWED_ORIGINS")
        .expect("allowed_origins must be set")
        .parse()
        .expect("Error parsing allowed origins")];

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods(vec![Method::GET, Method::PUT, Method::DELETE]);

    Router::<()>::new()
        .route("/files/{id}/{file_name}", delete(delete_file))
        .route("/files/{id}/{file_name}", get(get_files))
        .route("/health", get(health))
        .route("/files/{id}/{file_name}", put(upload_file))
        .route("/files/{id}/{file_name}/metadata", get(get_metadata))
        // axum's default body limit is 2MB; upload needs room for real files.
        .layer(DefaultBodyLimit::disable())
        .layer(cors)
}
