use axum::{
    response::{ Response, IntoResponse },
    http::StatusCode,
};
use dotenvy::dotenv;

mod routes;
mod handlers;
mod middleware;

enum ResponseError {
    NotFound,
    InternalError,
    FileExtError,
    InvalidFileName,
    InvalidId,
    Unauthorized,
    InvalidUrl,
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> Response {
        match self {
            ResponseError::NotFound => (StatusCode::NOT_FOUND, "Error 404 not found".to_string()).into_response(),
            ResponseError::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, "Error 500 internal error".to_string()).into_response(),
            ResponseError::FileExtError => (StatusCode::NOT_ACCEPTABLE, "File extension not acceptable".to_string()).into_response(),
            ResponseError::InvalidId => (StatusCode::NOT_ACCEPTABLE, "Id is not valid".to_string()).into_response(),
            ResponseError::InvalidFileName => (StatusCode::NOT_ACCEPTABLE, "File name is not valid".to_string()).into_response(),
            ResponseError::Unauthorized => (StatusCode::UNAUTHORIZED, "Error, auth is not valid".to_string()).into_response(),
            ResponseError::InvalidUrl => (StatusCode::UNAUTHORIZED, "URL is not valid".to_string()).into_response(),
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
   let app = routes::router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

