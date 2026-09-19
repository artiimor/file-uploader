
use axum::{
    response::{IntoResponse, Response},
    middleware::{self, Next},
    extract::{Request, Extension},
    http,
};
use crate::ResponseError;

pub async fn auth_bearer_token(req: Request, next: Next) -> Result<Response, ResponseError> {
    let auth_header = req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let bearer = std::env::var("API_KEY")
        .expect("API_KEY must be set");
    let bearer = format!("Bearer {}", bearer);

    if auth_header.unwrap() != bearer {
        println!("Error en la compararcion");
        return Err(ResponseError::Unauthorized);
    }

    Ok(next.run(req).await)
}
