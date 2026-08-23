use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use crate::jwt::generate_jwt_upload_token;

pub async fn get_upload_url() -> impl IntoResponse {
    let user_id = "some-user-id";
    let token = generate_jwt_upload_token(user_id);

    Json(json!({"upload_url": token.to_string()}))
}
