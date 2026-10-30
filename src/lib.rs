use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

mod handlers;
mod routes;

#[derive(PartialEq, Debug)]
pub enum ResponseError {
    NotFound,
    InternalError,
    FileExtError,
    InvalidFileName,
    InvalidId,
    Unauthorized,
    InvalidUrl,
    MissingFile,
    TooLarge,
    EmptyMultipart,
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> Response {
        match self {
            ResponseError::NotFound => {
                (StatusCode::NOT_FOUND, "Error 404 not found".to_string()).into_response()
            }
            ResponseError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error 500 internal error".to_string(),
            ).into_response(),
            ResponseError::FileExtError => (
                StatusCode::NOT_ACCEPTABLE,
                "File extension not acceptable".to_string(),
            ).into_response(),
            ResponseError::InvalidId => {
                (StatusCode::BAD_REQUEST, "Id is not valid".to_string()).into_response()
            }
            ResponseError::InvalidFileName => (
                StatusCode::BAD_REQUEST,
                "File name is not valid".to_string(),
            ).into_response(),
            ResponseError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Error, auth is not valid".to_string(),
            ).into_response(),
            ResponseError::InvalidUrl => {
                (StatusCode::UNAUTHORIZED, "URL is not valid".to_string()).into_response()
            }
            ResponseError::MissingFile => (
                StatusCode::BAD_REQUEST,
                "No file found in the request".to_string(),
            ).into_response(),
            ResponseError::TooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "File exceeded the limit!".to_string()
            ).into_response(),
            ResponseError::EmptyMultipart => (
                StatusCode::BAD_REQUEST,
                "Missing multipart".to_string(),
            ).into_response(),
        }
    }
}

pub fn app() -> axum::Router {
    routes::router()
}
