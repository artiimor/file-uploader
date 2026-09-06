use axum::{
    response::{ Response, IntoResponse },
    extract::Path,
};

use crate::ResponseError;

pub async fn health() -> Response {
    "igbbmn".to_string().into_response()
}

pub async fn get_files(Path(photo_path): Path<String>) -> String {
    // TODO download or error
    photo_path
}

pub async fn upload_file(Path(photo_path): Path<String>) -> ResponseError {
    // TODO Upload file
    ResponseError::Upload(format!("Tried to upload file {photo_path}").to_string())
}
