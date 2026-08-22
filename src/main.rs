use axum::response::IntoResponse;
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};
use routes::create_routes;

mod routes;
mod handlers;

#[derive(Debug)]
enum ApiError {
    NotFound, // 404
    InvalidInput(String), // 400
    InternalServerError,// 500
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let(status, error_message) = match self {
            ApiError::NotFound => (
                StatusCode::NOT_FOUND, "Not found".to_string()),
            ApiError::InvalidInput(msg) => (
                StatusCode::BAD_REQUEST, msg,
            ),
            ApiError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Json(json!{{
            "error": error_message,
        }});

        (status, body).into_response()
    }
}

#[tokio::main]
async fn main() {
    let app = create_routes();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Error binding TCP listener");

    println!("Server running on localhost:3000");

    axum::serve(listener, app)
        .await
        .expect("Error serving app");
}
