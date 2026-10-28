use axum::{ routing::{get, put, delete},
    Router,
    extract::DefaultBodyLimit,
};
use tower_http::cors::CorsLayer;
use crate::handlers::photos::{
    health,
    get_files,
    upload_file,
    delete_file,
    get_metadata,
};

pub fn router() -> Router{
    Router::<()>::new()
        .route("/files/{id}/{file_name}", delete(delete_file))
        .route("/files/{id}/{file_name}", get(get_files))
        .route("/health", get(health))
        .route("/files/{id}/{file_name}", put(upload_file))
        .route("/files/{id}/{file_name}/metadata", get(get_metadata))
        // axum's default body limit is 2MB; upload needs room for real files.
        .layer(DefaultBodyLimit::disable())
        .layer(CorsLayer::permissive())
}
