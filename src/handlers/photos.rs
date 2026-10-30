use crate::ResponseError;
use axum::{
    body::Body,
    extract::{FromRequest, Multipart, Path, Query, Request},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use jsonwebtoken::{DecodingKey, Validation, decode};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path as StdPath;
use std::sync::LazyLock;
use tokio::fs::File as TokioFile;
use tokio::fs::{metadata, remove_file};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String, // user id
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

pub async fn get_files(
    Path((id, file_name)): Path<(String, String)>,
    Query(params): Query<Params>,
) -> Result<Response, ResponseError> {
    check_jwt_token(&params.token, &id, "download", Some(file_name.clone()))?;

    check_file_name_regex(&file_name)?;
    check_id_regex(&id)?;

    let path = format!("files/{id}/{file_name}");
    let ext = StdPath::new(&file_name)
        .extension()
        .ok_or(ResponseError::FileExtError)?;
    let file = TokioFile::open(&path)
        .await
        .map_err(|_| ResponseError::NotFound)?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let headers = [
        (
            header::CONTENT_TYPE,
            format!("image/{}", ext.to_str().unwrap_or("octet-stream")),
        ),
        (
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{file_name}\""),
        ),
    ];

    Ok((headers, body).into_response())
}

pub async fn upload_file(
    Path((id, file_name)): Path<(String, String)>,
    Query(params): Query<Params>,
    request: Request,
) -> Result<Response, ResponseError> {
    check_jwt_token(&params.token, &id, "upload", Some(file_name.clone()))?;
    check_id_regex(&id)?;

    let is_multipart = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            s.split(';')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("multipart/form-data")
        }) // may god help us with string comparations
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
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|_| ResponseError::InternalError)?;
    }

    TokioFile::create(&path)
        .await
        .map_err(|_| ResponseError::InternalError)
}

pub async fn delete_file(
    Path((id, file_name)): Path<(String, String)>,
    Query(params): Query<Params>,
) -> Result<Response, ResponseError> {
    check_jwt_token(&params.token, &id, "delete", Some(file_name.clone()))?;
    check_file_name_regex(&file_name)?;
    check_id_regex(&id)?;

    let file_path = format!("files/{id}/{file_name}");
    match remove_file(&file_path).await {
        Ok(()) => Ok((
            StatusCode::OK,
            format!("file {file_path} deleted successfully!"),
        )
            .into_response()),
        Err(_) => Err(ResponseError::NotFound),
    }
}

pub async fn get_metadata(
    Path((id, file_name)): Path<(String, String)>,
    Query(params): Query<Params>,
) -> Result<Response, ResponseError> {
    check_jwt_token(&params.token, &id, "metadata", Some(file_name.clone()))?;
    check_file_name_regex(&file_name)?;
    check_id_regex(&id)?;

    let file_path = format!("files/{id}/{file_name}");
    let metadata = metadata(file_path)
        .await
        .map_err(|_| ResponseError::NotFound)?;

    Ok((
        StatusCode::OK,
        format!("The file size is {} bytes!", metadata.len()),
    )
        .into_response())
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

fn check_jwt_token(
    token: &String,
    user_id: &String,
    scope: &str,
    file_name: Option<String>,
) -> Result<(), ResponseError> {
    let token_secret = std::env::var("JWT_SECRET").expect("jwt_secret must be set");

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&token_secret.into_bytes()),
        &Validation::default(),
    )
    .map_err(|_| ResponseError::Unauthorized)?;

    if token_data.claims.scope != scope
        || token_data.claims.sub != *user_id
        || token_data.claims.file_name.unwrap_or_default() != file_name.unwrap_or_default()
    {
        return Err(ResponseError::InvalidUrl);
    }

    Ok(())
}

async fn handle_multipart_upload(
    id: &String,
    file_name: &String,
    multipart: &mut Multipart,
) -> Result<(), ResponseError> {
    check_file_name_regex(file_name)?;
    let tmp_file_name = format!("{file_name}.tmp");
    let tmp_path = format!("files/{id}/{tmp_file_name}");

    if let Err(e) = write_data_in_file(tmp_file_name.clone(), id.to_string(), multipart).await {
        let _ = remove_file(&tmp_path).await;

        return Err(e);
    }

    if let Err(_) = tokio::fs::rename(tmp_path.clone(), format!("files/{id}/{file_name}")).await {
        let _ = remove_file(&tmp_path).await;
        return Err(ResponseError::InternalError);
    }
    Ok(())
}

async fn write_data_in_file(tmp_file_name: String,
                            id: String,
                            multipart: &mut axum::extract::Multipart) ->
                            Result<(), ResponseError> {

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| ResponseError::InternalError)?
    {
        if field.file_name().is_none() {
            continue;
        }

        let tmp_path = format!("files/{id}/{tmp_file_name}");
        let mut tmp_file = create_file(&tmp_path).await?;

        // Write the data
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|_| ResponseError::InternalError)?
        {
            tmp_file.write_all(&chunk)
                .await
                .map_err(|_| ResponseError::InternalError)?;
        }
        return Ok(());
    }
    Err(ResponseError::MissingFile)
}

async fn handle_direct_upload(
    id: &String,
    file_name: &String,
    request: Request,
) -> Result<(), ResponseError> {
    check_file_name_regex(file_name)?;
    let path = format!("files/{id}/{file_name}");
    let tmp_path = format!("{path}.tmp");

    if let Err(_) = stream_write_in_path(request, tmp_path.clone()).await {
        let _ = remove_file(&tmp_path).await;
        return Err(ResponseError::InternalError);
    }

    if let Err(_) = tokio::fs::rename(tmp_path.clone(), path).await {
        let _ = remove_file(&tmp_path.clone()).await;
        return Err(ResponseError::InternalError);
    }
    Ok(())
}

async fn stream_write_in_path(request: Request, tmp_path: String) -> Result<(), ResponseError> {
    let mut file = create_file(&tmp_path).await?;
    let mut stream = request.into_body().into_data_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ResponseError::InternalError)?;
        file.write_all(&chunk)
            .await
            .map_err(|_| ResponseError::InternalError)?;
    }

    Ok(())
}

static ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_-]+$").expect("id regex"));

static FILE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_-]+\.[A-Za-z0-9]+$").expect("file regex"));

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header, encode, get_current_timestamp};
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            dotenvy::dotenv().ok();
        });
    }

    fn generate_jwt_token(
        user_id: &str,
        scope: &str,
        file_name: Option<String>,
    ) -> Result<String, ResponseError> {
        let token_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        let claims = Claims {
            sub: user_id.to_owned(),
            exp: get_current_timestamp() + 300, // 5 mins alive
            scope: scope.to_string(),
            file_name,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(token_secret.as_ref()),
        )
        .map_err(|_| ResponseError::InternalError)?;

        Ok(token)
    }

    #[test]
    fn file_name_regex_is_correct() {
        assert_eq!(check_file_name_regex("file.txt"), Ok(()));
        assert_eq!(
            check_file_name_regex("folder/file.txt"),
            Err(ResponseError::InvalidFileName)
        );
        assert_eq!(
            check_file_name_regex("../.."),
            Err(ResponseError::InvalidFileName)
        );
    }

    #[test]
    fn id_regex_is_correct() {
        assert_eq!(check_id_regex("user_id_wololo"), Ok(()));
        assert_eq!(check_id_regex(",,/file.txt"), Err(ResponseError::InvalidId));
        assert_eq!(check_id_regex("../.."), Err(ResponseError::InvalidId));
        assert_eq!(check_id_regex("id.nn"), Err(ResponseError::InvalidId));
    }

    #[test]
    fn check_jwt_token_works_when_it_matchs() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";
        let token = generate_jwt_token(user_id, scope, None).unwrap();

        assert_eq!(
            check_jwt_token(&token, &user_id.to_string(), scope, None),
            Ok(())
        );
    }

    #[test]
    fn check_jwt_token_fails_when_it_does_not_match() {
        setup();

        let user_id = "test_user_id";
        let scope = "upload";
        let token = generate_jwt_token(user_id, scope, None).unwrap();
        let other_scope = "other_scope";

        assert_eq!(
            check_jwt_token(&token, &user_id.to_string(), other_scope, None),
            Err(ResponseError::InvalidUrl)
        );
    }

    #[test]
    fn check_jwt_token_fails_when_sub_does_not_match() {
        setup();

        let user_id = "test_user_id";
        let other_user_id = "other_user_id";
        let scope = "download";
        let token = generate_jwt_token(user_id, scope, None).unwrap();

        assert_eq!(
            check_jwt_token(&token, &other_user_id.to_string(), scope, None),
            Err(ResponseError::InvalidUrl)
        );
    }

    #[test]
    fn check_jwt_token_fails_with_malformed_token() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";
        let token = "not.a.valid.jwt".to_string();

        assert_eq!(
            check_jwt_token(&token, &user_id.to_string(), scope, None),
            Err(ResponseError::Unauthorized)
        );
    }

    #[test]
    fn check_jwt_token_works_when_file_name_matchs() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";
        let file_name = "file.txt";
        let token = generate_jwt_token(user_id, scope, Some(file_name.to_string())).unwrap();

        assert_eq!(
            check_jwt_token(
                &token,
                &user_id.to_string(),
                scope,
                Some(file_name.to_string())
            ),
            Ok(())
        );
    }

    #[test]
    fn check_jwt_token_fails_when_file_name_does_not_match() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";
        let token = generate_jwt_token(user_id, scope, Some("file.txt".to_string())).unwrap();

        assert_eq!(
            check_jwt_token(
                &token,
                &user_id.to_string(),
                scope,
                Some("other_file.txt".to_string())
            ),
            Err(ResponseError::InvalidUrl)
        );
    }

    #[test]
    fn check_jwt_token_fails_when_file_name_is_missing() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";
        let token = generate_jwt_token(user_id, scope, Some("file.txt".to_string())).unwrap();

        assert_eq!(
            check_jwt_token(&token, &user_id.to_string(), scope, None),
            Err(ResponseError::InvalidUrl)
        );
    }

    #[test]
    fn check_jwt_token_fails_when_token_is_expired() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";
        let token_secret = std::env::var("JWT_SECRET").unwrap();

        // Validation::default() allows 60s of leeway, so expire well before that.
        let claims = Claims {
            sub: user_id.to_owned(),
            exp: get_current_timestamp() - 3600,
            scope: scope.to_string(),
            file_name: None,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(token_secret.as_ref()),
        )
        .unwrap();

        assert_eq!(
            check_jwt_token(&token, &user_id.to_string(), scope, None),
            Err(ResponseError::Unauthorized)
        );
    }

    #[test]
    fn check_jwt_token_fails_with_wrong_secret() {
        setup();

        let user_id = "test_user_id";
        let scope = "download";

        let claims = Claims {
            sub: user_id.to_owned(),
            exp: get_current_timestamp() + 300,
            scope: scope.to_string(),
            file_name: None,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"not-the-real-secret"),
        )
        .unwrap();

        assert_eq!(
            check_jwt_token(&token, &user_id.to_string(), scope, None),
            Err(ResponseError::Unauthorized)
        );
    }
}
