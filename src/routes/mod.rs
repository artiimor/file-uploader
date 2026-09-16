use axum::{
    routing::{get, put},
    Router,
};

use crate::handlers::photos::{
    health,
    get_files,
    upload_file,
    delete_file,
    get_metadata,
    get_download_url,
};

pub fn router() -> Router{
    Router::<()>::new()
        .route("/health", get(health))
        .route("/files/{id}", put(upload_file))
        .route("/files/{id}/{file_name}", get(get_files).delete(delete_file))
        .route("/files/{id}/{file_name}/metadata", get(get_metadata))
        .route("/download_url/{user_id}/{file_name}", get(get_download_url))
}

