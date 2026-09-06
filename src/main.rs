use axum::{
    routing::{get, post},
    Router,
    response::{ Response, IntoResponse },
    http::StatusCode,
    extract::Path,
};


enum ResponseError {
    NotFound,
    InternalError,
    Wololo,
    Upload(String),
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> Response {
        match self {
        ResponseError::NotFound => (StatusCode::NOT_FOUND, "Error 404 not found".to_string()).into_response(),
        ResponseError::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, "Error 500 internal error".to_string()).into_response(),
        ResponseError::Wololo => (StatusCode::ACCEPTED, "Wololo".to_string()).into_response(),
        ResponseError::Upload(message) => (StatusCode::NOT_ACCEPTABLE, message).into_response(),
        }
    }
}

#[tokio::main]
async fn main() {
   let app = Router::<()>::new()
        .route("/health", get(health))
        .route("/files/{photo_path}", get(get_files).post(upload_file));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn health() -> Response {
    "igbbmn".to_string().into_response()
}

async fn get_files(Path(photo_path): Path<String>) -> String {
    // TODO download or error
    photo_path
}

async fn upload_file(Path(photo_path): Path<String>) -> ResponseError {
    // TODO Upload file
    ResponseError::Upload(format!("Tried to upload file {photo_path}").to_string())
}
