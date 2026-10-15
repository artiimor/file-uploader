use axum::{
    response::{ Response, IntoResponse },
    extract::{Path, Multipart, Query},
    http::{header, StatusCode},
    body::Body,
};
use crate::ResponseError;
use tokio_util::io::ReaderStream;
use tokio::fs::File as TokioFile;
use tokio::fs::{remove_file, metadata};
use tokio::io::AsyncWriteExt;
use std::path::Path as StdPath;
use std::sync::LazyLock;
use regex::Regex;
use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, decode, get_current_timestamp, Header, EncodingKey, DecodingKey, Validation};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,   // user id
    exp: u64,
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
    check_jwt_token(&params.token, &id, "download")?;

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
    check_jwt_token(&params.token, &id, "upload")?;

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

    TokioFile::create(&path).await.map_err(|_| ResponseError::InternalError)
}

pub async fn delete_file(Path((id, file_name)): Path<(String, String)>) -> Result<Response, ResponseError> {
    check_file_name_regex(&file_name)?;
    check_id_regex(&id)?;

    let file_path = format!("files/{id}/{file_name}");
    match remove_file(&file_path).await {
        Ok(()) => Ok((StatusCode::OK, format!("file {file_path} deleted successfully!")).into_response()),
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
    exp: get_current_timestamp() + 300, // 5 mins alive
    scope: scope.to_string(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(token_secret.as_ref()),
    ).map_err(|_| ResponseError::InternalError)?;

    Ok(token)
}

fn check_file_name_regex(file_name: &str) -> Result<(), ResponseError> {
    if FILE_RE.is_match(file_name) {
        Ok(())
    } else {
        Err(ResponseError::InvalidFileName)
    }
}

fn check_id_regex(id: &str) -> Result<(), ResponseError> {
    if ID_RE.is_match(id) {
        Ok(())
    } else {
        Err(ResponseError::InvalidId)
    }
}

fn check_jwt_token(token: &String, user_id: &String, scope: &str) -> Result<(), ResponseError> {
    let token_secret = std::env::var("JWT_SECRET")
        .expect("jwt_secret must be set");

    let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&token_secret.into_bytes()),
            &Validation::default(),
        ).map_err(|_| ResponseError::Unauthorized)?;

    if token_data.claims.scope != scope ||
       token_data.claims.sub != *user_id {
        return Err(ResponseError::InvalidUrl);
    }

    Ok(())
}

static ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9_-]+$").expect("id regex")
});

static FILE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9_-]+\.[A-Za-z0-9]+$").expect("file regex")
});

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            dotenvy::dotenv().ok();
        });
    }

    #[test]
    fn file_name_regex_is_correct() {
        assert_eq!(check_file_name_regex("file.txt"), Ok(()));
        assert_eq!(check_file_name_regex("folder/file.txt"), Err(ResponseError::InvalidFileName));
        assert_eq!(check_file_name_regex("../.."), Err(ResponseError::InvalidFileName));
    }

    #[test]
    fn id_regex_is_correct() {
        assert_eq!(check_id_regex("user_id_wololo"), Ok(()));
        assert_eq!(check_id_regex(",,/file.txt"), Err(ResponseError::InvalidId));
        assert_eq!(check_id_regex("../.."), Err(ResponseError::InvalidId));
        assert_eq!(check_id_regex("id.nn"), Err(ResponseError::InvalidId));

    }

    #[test]
    fn generated_jwt_token_with_upload_scope() {
        setup();

        let user_id = "test_user_id";
        let scope = "upload";

        let token_secret = std::env::var("JWT_SECRET").unwrap();

        let token = generate_jwt_token(user_id, scope);
        let token_data = decode::<Claims>(
            token.unwrap(),
            &DecodingKey::from_secret(&token_secret.into_bytes()),
            &Validation::default(),).unwrap();

        assert_eq!(token_data.claims.sub, user_id);
        assert_eq!(token_data.claims.scope, scope);
    }

    #[test]
    fn generated_jwt_token_with_download_scope() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";

        let token_secret = std::env::var("JWT_SECRET").unwrap();

        let token = generate_jwt_token(user_id, scope);
        let token_data = decode::<Claims>(
            token.unwrap(),
            &DecodingKey::from_secret(&token_secret.into_bytes()),
            &Validation::default(),).unwrap();

        assert_eq!(token_data.claims.sub, user_id);
        assert_eq!(token_data.claims.scope, scope);
    }

    #[test]
    fn generated_jwt_token_fails_with_random_scope() {
        setup();

        let user_id = "test_user_id";
        let scope = "asdasd";

        let token = generate_jwt_token(user_id, scope);
        assert_eq!(token, Err(ResponseError::InternalError))
    }

    #[test]
    fn check_jwt_token_works_when_it_matchs() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";
        let token = generate_jwt_token(user_id, scope).unwrap();

        assert_eq!(check_jwt_token(&token, &user_id.to_string(), scope), Ok(()));
    }

    #[test]
    fn check_jwt_token_fails_when_it_does_not_match() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";
        let token = generate_jwt_token(user_id, scope).unwrap();
        let other_scope = "other_scope";

        assert_eq!(check_jwt_token(&token, &user_id.to_string(), other_scope), Err(ResponseError::InvalidUrl));
    }
}
