use axum::{
    routing::{get, put, delete},
    Router,
    middleware,
};
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
        .route("/upload_url/{user_id}", get(get_upload_url))
        .route("/download_url/{user_id}/{file_name}", get(get_download_url))
        .route("/files/{id}/{file_name}", delete(delete_file))
        .route_layer(middleware::from_fn(auth_bearer_token))
        .route("/files/{id}/{file_name}", get(get_files))
        .route("/health", get(health))
        .route("/files/{id}", put(upload_file))
        .route("/files/{id}/{file_name}/metadata", get(get_metadata))
}
