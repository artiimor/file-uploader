use crate::handlers::photos::{delete_file, get_files, get_metadata, health, upload_file};
use crate::AppConfig;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{Method, header},
    routing::{delete, get, put},
};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer};

pub fn router() -> Router {
    dotenvy::dotenv().ok();

    let allowed_origins = [std::env::var("ALLOWED_ORIGINS")
        .expect("allowed_origins must be set")
        .parse()
        .expect("Error parsing allowed origins")];

    let max_body_limit: usize = std::env::var("MAX_BODY_LIMIT")
        .expect("max_body_limit must be set")
        .parse()
        .expect("max_body_limit format was wrong");

    let config = AppConfig {
        files_root_path: std::env::var("FILES_ROOT_PATH")
            .expect("FILES_ROOT_PATH must be set"),
    };

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods(vec![Method::GET, Method::OPTIONS, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE]);

    Router::<AppConfig>::new()
        .route("/files/{id}/{file_name}", delete(delete_file))
        .route("/files/{id}/{file_name}", get(get_files))
        .route("/files/{id}/{file_name}", put(upload_file))
        .route("/files/{id}/{file_name}/metadata", get(get_metadata))
        .with_state(config)
        .route("/health", get(health))
        .layer(DefaultBodyLimit::max(max_body_limit * 1024 * 1024))
        .layer(RequestBodyLimitLayer::new(max_body_limit * 1024 * 1024))
        .layer(cors)
}
