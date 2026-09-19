
use axum::{
    response::{Response},
    middleware::{Next},
    extract::{Request},
    http,
};
use crate::ResponseError;

pub async fn auth_bearer_token(req: Request, next: Next) -> Result<Response, ResponseError> {
    let bearer = std::env::var("API_KEY")
        .expect("API_KEY must be set");
    let bearer = format!("Bearer {}", bearer);

    if let Some(auth_header) = req.headers().get(http::header::AUTHORIZATION) {
        if *auth_header == *bearer {
            return Ok(next.run(req).await);
       }
    }
    Err(ResponseError::Unauthorized)
}
