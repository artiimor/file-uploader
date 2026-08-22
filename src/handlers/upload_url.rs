use axum::response::IntoResponse;

pub async fn get_upload_url() -> impl IntoResponse {
    "https://www.example.com/upload"
}
