use axum::response::IntoResponse;
use axum::Json;
use axum::extract::Path;
use axum::extract::Multipart;
use serde_json::{json, Value};
use axum::http::StatusCode;
use axum::extract::multipart::Field;

use crate::jwt::decode_jwt_token;
use crate::jwt::Claims;
use chrono::DateTime;
use tokio::io::AsyncWriteExt;

pub async fn put_upload_file(
    Path(token): Path<String>,
    mut multipart: Multipart,
    ) -> impl IntoResponse {

    // Test the token
    let data = match decode_jwt_token(&token) {
        Ok(data) => data,
        Err(status) => return status.into_response(),
    };

    // Check token scope
    match check_data_scope(&data.claims) {
        Ok(()) => (),
        Err(status) => return status.into_response(),
    };

    // Write the file
    let field = match multipart.next_field().await {
        Ok(Some(field)) => field,
        Ok(None) => return StatusCode::BAD_REQUEST.into_response(),
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    if let Err(status) = save_upload(field, &data.claims).await {
        return status.into_response();
    }

    Json(json!({"sub": data.claims})).into_response()
}

fn check_data_scope(claims: &Claims) -> Result<(), StatusCode> {
    if claims.scope == "Upload" {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

async fn save_upload(mut field: Field<'_>, claims: &Claims) -> Result<(), StatusCode> {
    let content_type: &str = match field.content_type() {
        Some("image/jpeg") => "jpg",
        Some("image/png") => "png",
        Some("image/webp") => "webp",
        _ => return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE)
    };
    let formatted_date: String = match DateTime::from_timestamp(claims.iat as i64, 0) {
        Some(date) => date.format("%Y%m%d_%H%M%S").to_string(),
        None => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let file_dir: String = format!("file_uploader/{}", claims.sub);
    // Need to create the directory if does not exist
    tokio::fs::create_dir_all(&file_dir).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let file_name = format!("{file_dir}/{}_{formatted_date}.{content_type}", claims.sub);

    let mut file = tokio::fs::File::create(&file_name).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    loop {
        match field.chunk().await {
            Ok(Some(chunk)) => {
                if file.write_all(&chunk).await.is_err() {
                    let _ = tokio::fs::remove_file(&file_name).await;
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
            Ok(None) => break,
            Err(_) => {
                let _ = tokio::fs::remove_file(&file_name).await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok(())
}
