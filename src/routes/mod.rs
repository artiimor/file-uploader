use axum::{
    routing::{get, post},
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
        .route("/files/{photo_path}", get(get_files).post(upload_file))
}

