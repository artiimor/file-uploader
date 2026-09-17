use axum::{
    response::{ Response, IntoResponse },
    extract::{Path, Multipart, Query},
    http::{header, StatusCode},
    body::Body,
};
use tokio_util::io::ReaderStream;
use crate::ResponseError;
use tokio::fs::File as TokioFile;
use std::path::Path as StdPath;
use tokio::fs::remove_file;
use tokio::fs::metadata;
use regex::Regex;
use std::sync::LazyLock;
use tokio::io::AsyncWriteExt;
use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};
use jsonwebtoken::get_current_timestamp;
use dotenvy::dotenv;

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,   // user id
    exp: usize,
    scope: String,
}

#[derive(Deserialize)]
pub struct Params {
    token: String,
}

pub async fn health() -> Response {
    "igbbmn".to_string().into_response()
}

pub async fn get_download_url(Path((user_id, file_name)): Path<(String, String)>) -> Result<Response, ResponseError> {
    let token = generate_jwt_token(&user_id, "download")?;

    Ok(format!("/files/{user_id}/{file_name}?token={token}").into_response())
}

pub async fn get_upload_url(Path(user_id): Path<String>) -> Result<Response, ResponseError> {
    let token = generate_jwt_token(&user_id, "upload")?;

    Ok(format!("/files/{user_id}?token={token}").into_response())
}

pub async fn get_files(Path((id, file_name)): Path<(String, String)>,
                       Query(params): Query<Params>) -> Result<Response, ResponseError> {
    let token_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");

    // TODO extract verification method to function
    let token_data = decode::<Claims>(
            params.token,
            &DecodingKey::from_secret(&token_secret.into_bytes()),
            &Validation::default(),
        ).map_err(|_| ResponseError::Unauthorized)?;

    if token_data.claims.scope != "download" ||
       token_data.claims.sub != id ||
       token_data.claims.exp < get_current_timestamp() as usize {
        return Err(ResponseError::InvalidUrl);
    }

    check_file_name_regex(&file_name)?;
    check_id_regex(&id)?;

    let path = format!("files/{id}/{file_name}");
    let ext = StdPath::new(&file_name)
        .extension()
        .ok_or(ResponseError::FileExtError)?;
    let file = TokioFile::open(&path).await.map_err(|_| ResponseError::NotFound)?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let headers = [
        (header::CONTENT_TYPE, format!("application/{:?}", ext.to_str())), // TODO fix for all file type
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{file_name}\""),
        ),
    ];

    Ok((headers, body).into_response())
}

pub async fn upload_file(Path(id): Path<String>,
                         Query(params): Query<Params>,
                         mut multipart: Multipart) -> Result<Response, ResponseError> {
    let token_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");

    let token_data = decode::<Claims>(
            params.token,
            &DecodingKey::from_secret(&token_secret.into_bytes()),
            &Validation::default(),
        ).map_err(|_| ResponseError::Unauthorized)?;

    if token_data.claims.scope != "upload" ||
       token_data.claims.sub != id ||
       token_data.claims.exp < get_current_timestamp() as usize {
        return Err(ResponseError::InvalidUrl);
    }

    check_id_regex(&id)?;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| ResponseError::InternalError)? {

        let name = field.file_name().ok_or(ResponseError::InvalidFileName)?;
        let name = name.to_string();
        check_file_name_regex(&name)?;

        let path = format!("files/{id}/{name}");
        let mut file = create_file(&path).await?;

        // Write the data
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|_| ResponseError::InternalError)?
        {
            file.write_all(&chunk)
                .await
                .map_err(|_| ResponseError::InternalError)?;
        }
    }
    Ok("File uploaded!".to_string().into_response())
}

pub async fn create_file(path: &String) -> Result<TokioFile, ResponseError> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|_| ResponseError::InternalError)?;
    }

    Ok(TokioFile::create(&path).await.map_err(|_| ResponseError::InternalError)?)
}

pub async fn delete_file(Path((id, file_name)): Path<(String, String)>) -> Result<Response, ResponseError> {
    check_file_name_regex(&file_name)?;
    check_id_regex(&id)?;

    let file_path = format!("files/{id}/{file_name}");
    match remove_file(&file_path).await {
        Ok(_) => Ok((StatusCode::OK, format!("file {file_path} deleted successfully!")).into_response()),
        Err(_) => Err(ResponseError::NotFound),
    }
}

pub async fn get_metadata(Path((id, file_name)): Path<(String, String)>) -> Result<Response, ResponseError> {
    // TODO add jwt token

    check_file_name_regex(&file_name)?;
    check_id_regex(&id)?;

    let file_path = format!("files/{id}/{file_name}");
    let metadata = metadata(file_path).await.map_err(|_| ResponseError::NotFound)?;

    Ok((StatusCode::OK, format!("The file size is {} bytes!", metadata.len())).into_response())
}

fn generate_jwt_token(user_id: &str, scope: &str) -> Result<String, ResponseError> {
    let token_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");

    if scope != "upload" && scope != "download" {
        return Err(ResponseError::InternalError)
    }

    let claims = Claims {
    sub: user_id.to_owned(),
    exp: get_current_timestamp() as usize + 300, // 5 mins alive
    scope: scope.to_string(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(token_secret.as_ref()),
    ).map_err(|_| ResponseError::InternalError)?;

    Ok(token)
}

fn check_file_name_regex(file_name: &String) -> Result<(), ResponseError> {
    if FILE_RE.is_match(file_name) {
        Ok(())
    } else {
        Err(ResponseError::InvalidFileName)
    }
}

fn check_id_regex(id: &String) -> Result<(), ResponseError> {
    if ID_RE.is_match(id) {
        Ok(())
    } else {
        Err(ResponseError::InvalidId)
    }
}

static ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9_-]+$").expect("id regex")
});

static FILE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9_-]+\.[A-Za-z0-9]+$").expect("file regex")
});
