use axum::{
    response::{ Response, IntoResponse },
    extract::{ Path, Multipart, Query, Request, FromRequest },
    http::{ header, StatusCode },
    body::{ Body },
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
use jsonwebtoken::{decode, DecodingKey, Validation};
use futures_util::StreamExt;

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,   // user id
    exp: u64,
    scope: String,
    file_name: Option<String>,
}

#[derive(Deserialize)]
pub struct Params {
    token: String,
}

pub async fn health() -> Response {
    "igbbmn".to_string().into_response()
}

pub async fn get_files(Path((id, file_name)): Path<(String, String)>,
                       Query(params): Query<Params>) -> Result<Response, ResponseError> {
    check_jwt_token(&params.token, &id, "download", Some(file_name.clone()))?;

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
        (header::CONTENT_TYPE, format!("image/{}", ext.to_str().unwrap_or("octet-stream"))),
        (
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{file_name}\""),
        ),
    ];

    Ok((headers, body).into_response())
}

pub async fn upload_file(Path((id, file_name)): Path<(String, String)>,
                         Query(params): Query<Params>,
                         request: Request) -> Result<Response, ResponseError> {
    check_jwt_token(&params.token, &id, "upload", Some(file_name.clone()))?;
    check_id_regex(&id)?;

    let is_multipart = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or("").trim().eq_ignore_ascii_case("multipart/form-data")) // may god help us with string comparations
        .unwrap_or(false);

    if is_multipart {
        let mut multipart = Multipart::from_request(request, &())
            .await
            .map_err(|_| ResponseError::InternalError)?;
        handle_multipart_upload(&id, &file_name, &mut multipart).await?;
    } else {
        handle_direct_upload(&id, &file_name, request).await?;
    }

    Ok("File uploaded!".to_string().into_response())
}

pub async fn create_file(path: &String) -> Result<TokioFile, ResponseError> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|_| ResponseError::InternalError)?;
    }

    TokioFile::create(&path).await.map_err(|_| ResponseError::InternalError)
}

pub async fn delete_file(Path((id, file_name)): Path<(String, String)>,
                         Query(params): Query<Params>) -> Result<Response, ResponseError> {
    check_jwt_token(&params.token, &id, "delete", Some(file_name.clone()))?;
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

fn check_jwt_token(token: &String, user_id: &String, scope: &str, file_name: Option<String>) -> Result<(), ResponseError> {
    let token_secret = std::env::var("JWT_SECRET") .expect("jwt_secret must be set");

    let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&token_secret.into_bytes()),
            &Validation::default(),
        ).map_err(|_| ResponseError::Unauthorized)?;

    if token_data.claims.scope != scope ||
       token_data.claims.sub != *user_id ||
       token_data.claims.file_name.unwrap_or_default() != file_name.unwrap_or_default() {
        return Err(ResponseError::InvalidUrl);
    }

    Ok(())
}

async fn handle_multipart_upload(id: &String, file_name: &String, multipart: &mut Multipart) -> Result<(), ResponseError> {
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| ResponseError::InternalError)? {

        check_file_name_regex(&file_name)?;

        let path = format!("files/{id}/{file_name}");
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
    Ok(())
}

async fn handle_direct_upload(id: &String, file_name: &String, request: Request) -> Result<(), ResponseError> {
    check_file_name_regex(file_name)?;
    let path = format!("files/{id}/{file_name}");

    let mut file = create_file(&path).await?;

    let mut stream = request.into_body().into_data_stream();
    // TODO if it fails we must delete the file
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ResponseError::InternalError)?;
        file.write_all(&chunk).await.map_err(|_| ResponseError::InternalError)?;
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

        let token = generate_jwt_token(user_id, scope, None);
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
        let file_name = "file_name";

        let token_secret = std::env::var("JWT_SECRET").unwrap();

        let token = generate_jwt_token(user_id, scope, Some(file_name.to_string()));
        let token_data = decode::<Claims>(
            token.unwrap(),
            &DecodingKey::from_secret(&token_secret.into_bytes()),
            &Validation::default(),).unwrap();

        assert_eq!(token_data.claims.sub, user_id);
        assert_eq!(token_data.claims.scope, scope);
        assert_eq!(token_data.claims.file_name, Some(file_name.to_string()));
    }

    #[test]
    fn generated_jwt_token_fails_with_random_scope() {
        setup();

        let user_id = "test_user_id";
        let scope = "asdasd";

        let token = generate_jwt_token(user_id, scope, None);
        assert_eq!(token, Err(ResponseError::InternalError))
    }

    #[test]
    fn check_jwt_token_works_when_it_matchs() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";
        let token = generate_jwt_token(user_id, scope, None).unwrap();

        assert_eq!(check_jwt_token(&token, &user_id.to_string(), scope, None), Ok(()));
    }

    #[test]
    fn check_jwt_token_fails_when_it_does_not_match() {
        setup();

        let user_id = "test_user_id";
        let scope = "upload";
        let token = generate_jwt_token(user_id, scope, None).unwrap();
        let other_scope = "other_scope";

        assert_eq!(check_jwt_token(&token, &user_id.to_string(), other_scope, None), Err(ResponseError::InvalidUrl));
    }

    #[test]
    fn check_jwt_token_fails_when_sub_does_not_match() {
        setup();

        let user_id = "test_user_id";
        let other_user_id = "other_user_id";
        let scope = "download";
        let token = generate_jwt_token(user_id, scope, None).unwrap();

        assert_eq!(check_jwt_token(&token, &other_user_id.to_string(), scope, None), Err(ResponseError::InvalidUrl));
    }

    #[test]
    fn check_jwt_token_fails_with_malformed_token() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";
        let token = "not.a.valid.jwt".to_string();

        assert_eq!(check_jwt_token(&token, &user_id.to_string(), scope, None), Err(ResponseError::Unauthorized));
    }
}
