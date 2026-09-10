use axum::{
    routing::{get, put},
    Router,
};

use crate::handlers::photos::{
    health,
    get_files,
    upload_file,
};

pub fn router() -> Router{
    Router::<()>::new()
        .route("/health", get(health))
        .route("/files/{id}", put(upload_file))
        .route("/files/{id}/{file_name}", get(get_files))
}

