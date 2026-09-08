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
        .route("/files/{id}", get(get_files).put(upload_file))
}

