use axum::{
    Router,
    http::StatusCode,
    routing::get,
    response::{IntoResponse, Response},
    middleware::{self, Next},
    extract::Request,
};

fn api_key() -> String {
    std::env::var("API_KEY").expect("API_KEY must be set")
}

pub async fn auth(request: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = request.headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if auth_header == format!("Bearer {}", api_key()) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn handler() -> impl IntoResponse {
    "You are authorized!"
}
