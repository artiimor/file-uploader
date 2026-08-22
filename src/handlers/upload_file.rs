use axum::response::IntoResponse;
use axum::Json;
use axum::extract::Path;
use axum::extract::Multipart;
use serde_json::{json, Value};

use crate::jwt::decode_jwt_token;

pub async fn put_upload_file(
    Path(token): Path<String>,
    mut multipart: Multipart,
    ) -> impl IntoResponse {

    let data = match decode_jwt_token(&token) {
        Ok(data) => data,
        Err(status) => return status.into_response(),
    };

    Json(json!({"sub": data.claims})).into_response()
}
