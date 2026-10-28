use axum::{ routing::{get, put, delete},
    Router,
    middleware,
    extract::DefaultBodyLimit,
};
use tower_http::cors::CorsLayer;
use crate::middleware::auth::auth_bearer_token;
use crate::handlers::photos::{
    health,
    get_files,
    upload_file,
    delete_file,
    get_metadata,
    get_download_url,
    get_upload_url,
};

pub fn router() -> Router{
    Router::<()>::new()
        .route("/upload_url/{user_id}/{file_name}", get(get_upload_url))
        .route("/download_url/{user_id}/{file_name}", get(get_download_url))
        .route_layer(middleware::from_fn(auth_bearer_token))
        .route("/files/{id}/{file_name}", delete(delete_file))
        .route("/files/{id}/{file_name}", get(get_files))
        .route("/health", get(health))
        .route("/files/{id}/{file_name}", put(upload_file))
        .route("/files/{id}/{file_name}/metadata", get(get_metadata))
        // axum's default body limit is 2MB; upload needs room for real files.
        .layer(DefaultBodyLimit::disable())
        .layer(CorsLayer::permissive())
}
