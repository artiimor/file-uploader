use file_uploader::app;
use jsonwebtoken::{EncodingKey, Header, encode, get_current_timestamp};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::test]
async fn test_health_check_integration() {
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client
        .get(format!("http://0.0.0.0:{port}/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().await.unwrap();
    assert_eq!(body, "igbbmn");
}

#[tokio::test]
async fn test_download_unexistent_file() {
    dotenvy::dotenv().ok();

    let user_id = "download_test_user_id";
    let file_name = "unexistent_file.txt";
    let token = generate_jwt_token(user_id, "download", Some(file_name.to_string()));

    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let download_res = client
        .get(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(download_res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_download_with_wrong_token_file() {
    dotenvy::dotenv().ok();

    let user_id = "download_test_user_id2";
    let other_file_name = "error.txt";
    let file_name = "file.txt";
    let token = generate_jwt_token(user_id, "download", Some(other_file_name.to_string()));

    create_file_and_dir(user_id, file_name).await;
    create_file_and_dir(user_id, other_file_name).await;

    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let download_res = client
        .get(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(download_res.status(), StatusCode::UNAUTHORIZED);

    tokio::fs::remove_file(format!("files/{user_id}/{file_name}"))
        .await
        .unwrap();
    tokio::fs::remove_file(format!("files/{user_id}/{other_file_name}"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_download_with_wrong_token_user() {
    dotenvy::dotenv().ok();

    let user_id = "download_test_user_id";
    let other_user_id = "other_user";
    let file_name = "file.txt";
    let token = generate_jwt_token(other_user_id, "download", Some(file_name.to_string()));

    create_file_and_dir(user_id, file_name).await;
    create_file_and_dir(other_user_id, file_name).await;

    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let download_res = client
        .get(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(download_res.status(), StatusCode::UNAUTHORIZED);

    tokio::fs::remove_file(format!("files/{user_id}/{file_name}"))
        .await
        .unwrap();
    tokio::fs::remove_file(format!("files/{other_user_id}/{file_name}"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_upload_with_generated_url() {
    dotenvy::dotenv().ok();

    let user_id = "upload_test_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(b"hello world".to_vec()).file_name(file_name),
    );
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(upload_res.status(), StatusCode::OK);
    assert!(tokio::fs::try_exists(&path).await.unwrap());
    assert_eq!(tokio::fs::read(&path).await.unwrap(), b"hello world");
    tokio::fs::remove_file(format!("files/{user_id}/{file_name}"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_direct_upload_with_generated_url() {
    dotenvy::dotenv().ok();

    let user_id = "direct_upload_test_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .body(b"hello world".to_vec())
        .send()
        .await
        .unwrap();

    assert_eq!(upload_res.status(), StatusCode::OK);
    assert!(tokio::fs::try_exists(&path).await.unwrap());
    assert_eq!(tokio::fs::read(&path).await.unwrap(), b"hello world");
    tokio::fs::remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn test_delete_file() {
    dotenvy::dotenv().ok();

    let user_id = "delete_test_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    let port = spawn_serve().await.unwrap().port();
    let token = generate_jwt_token(user_id, "delete", Some(file_name.to_string()));

    create_file_and_dir(user_id, file_name).await;

    let client = Client::new();
    let delete_res = client
        .delete(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(delete_res.status(), StatusCode::OK);
    assert!(!tokio::fs::try_exists(&path).await.unwrap());
}

#[tokio::test]
async fn test_delete_file_with_wrong_token() {
    dotenvy::dotenv().ok();

    let user_id = "delete_wrong_token_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    let port = spawn_serve().await.unwrap().port();

    create_file_and_dir(user_id, file_name).await;

    let bad_tokens = [
        generate_jwt_token(user_id, "upload", Some(file_name.to_string())),
        generate_jwt_token(user_id, "delete", Some("other_file.txt".to_string())),
        generate_jwt_token("other_user", "delete", Some(file_name.to_string())),
        "not.a.valid.jwt".to_string(),
    ];

    let client = Client::new();
    for token in bad_tokens {
        let delete_res = client
            .delete(format!(
                "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(delete_res.status(), StatusCode::UNAUTHORIZED);
    }

    assert!(tokio::fs::try_exists(&path).await.unwrap());
}

#[tokio::test]
async fn test_get_metadata() {
    dotenvy::dotenv().ok();

    let user_id = "metadata_test_user_id";
    let file_name = "my_file.txt";
    let token = generate_jwt_token(user_id, "metadata", Some(file_name.to_string()));

    create_file_and_dir(user_id, file_name).await;

    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client
        .get(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}/metadata?token={token}"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().await.unwrap();
    assert_eq!(body, "The file size is 5 bytes!");

    tokio::fs::remove_file(format!("files/{user_id}/{file_name}"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_get_metadata_with_wrong_token() {
    dotenvy::dotenv().ok();

    let user_id = "metadata_wrong_token_user_id";
    let file_name = "my_file.txt";
    let port = spawn_serve().await.unwrap().port();

    create_file_and_dir(user_id, file_name).await;

    let bad_tokens = [
        generate_jwt_token(user_id, "download", Some(file_name.to_string())),
        generate_jwt_token(user_id, "metadata", Some("other_file.txt".to_string())),
        generate_jwt_token("other_user", "metadata", Some(file_name.to_string())),
        "not.a.valid.jwt".to_string(),
    ];

    let client = Client::new();
    for token in bad_tokens {
        let res = client
            .get(format!(
                "http://localhost:{port}/files/{user_id}/{file_name}/metadata?token={token}"
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    tokio::fs::remove_file(format!("files/{user_id}/{file_name}"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_get_metadata_unexistent_file() {
    dotenvy::dotenv().ok();

    let user_id = "metadata_test_user_id2";
    let file_name = "unexistent_file.txt";
    let token = generate_jwt_token(user_id, "metadata", Some(file_name.to_string()));

    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client
        .get(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}/metadata?token={token}"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

async fn spawn_serve() -> Result<SocketAddr, std::io::Error> {
    let app = app();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    let addr = listener.local_addr();

    tokio::spawn(async move { axum::serve(listener, app.clone()).await.unwrap() });

    addr
}

async fn create_file_and_dir(user_id: &str, file_name: &str) {
    // If test fails is nevet deleted
    // TODO create temp dile with impl of drop that removes it always
    tokio::fs::create_dir_all(format!("files/{user_id}"))
        .await
        .unwrap();
    tokio::fs::write(format!("files/{user_id}/{file_name}"), b"hello")
        .await
        .unwrap();
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String, // user id
    exp: u64,
    scope: String,
    file_name: Option<String>,
}

fn generate_jwt_token(user_id: &str, scope: &str, file_name: Option<String>) -> String {
    let token_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let claims = Claims {
        sub: user_id.to_owned(),
        exp: get_current_timestamp() + 300, // 5 mins alive
        scope: scope.to_string(),
        file_name,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(token_secret.as_ref()),
    )
    .unwrap()
}

// Status code tests: these assert the *correct* HTTP code for each error case.

#[tokio::test]
async fn test_missing_token_is_unauthorized() {
    dotenvy::dotenv().ok();

    let user_id = "missing_token_user_id";
    let file_name = "my_file.txt";
    let port = spawn_serve().await.unwrap().port();
    let url = format!("http://localhost:{port}/files/{user_id}/{file_name}");

    let client = Client::new();
    let requests = [
        client.get(&url),
        client.put(&url).body(b"hello".to_vec()),
        client.delete(&url),
        client.get(format!("{url}/metadata")),
    ];

    for req in requests {
        let res = req.send().await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn test_invalid_id_is_bad_request() {
    dotenvy::dotenv().ok();

    let user_id = "bad.id";
    let file_name = "my_file.txt";
    let token = generate_jwt_token(user_id, "download", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client
        .get(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_invalid_file_name_is_bad_request() {
    dotenvy::dotenv().ok();

    let user_id = "invalid_file_name_user_id";
    let file_name = "no_extension";
    let token = generate_jwt_token(user_id, "download", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client
        .get(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_direct_upload_over_limit_is_payload_too_large() {
    dotenvy::dotenv().ok();

    let user_id = "direct_upload_over_limit_user_id";
    let file_name = "big_file.bin";
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .body(over_limit_body())
        .send()
        .await
        .unwrap();

    let status = res.status();
    tokio::fs::remove_file(format!("files/{user_id}/{file_name}"))
        .await
        .ok();

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn test_multipart_upload_over_limit_is_payload_too_large() {
    dotenvy::dotenv().ok();

    let user_id = "multipart_upload_over_limit_user_id";
    let file_name = "big_file.bin";
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(over_limit_body()).file_name(file_name),
    );
    let res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let status = res.status();
    tokio::fs::remove_file(format!("files/{user_id}/{file_name}"))
        .await
        .ok();

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn test_malformed_multipart_is_bad_request() {
    dotenvy::dotenv().ok();

    let user_id = "malformed_multipart_user_id";
    let file_name = "my_file.txt";
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .header("content-type", "multipart/form-data") // no boundary
        .body(b"hello".to_vec())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_empty_multipart_is_bad_request() {
    dotenvy::dotenv().ok();

    let user_id = "empty_multipart_user_id";
    let file_name = "my_file.txt";
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(reqwest::multipart::Form::new())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_multipart_first_two_files_keeps_first() {
    dotenvy::dotenv().ok();

    let user_id = "multipart_first_two_files_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"first".to_vec()).file_name(file_name),
        )
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"second".to_vec()).file_name(file_name),
        );
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(form)
        .send()
        .await
        .unwrap();

    // Read and clean up before asserting, so a failing assert leaves no garbage
    let content = tokio::fs::read(&path).await.unwrap();
    tokio::fs::remove_file(&path).await.unwrap();

    assert_eq!(upload_res.status(), StatusCode::OK);
    assert_eq!(content, b"first");
}

#[tokio::test]
async fn test_multipart_first_text_field_before_file_is_ignored() {
    dotenvy::dotenv().ok();

    let user_id = "multipart_first_text_before_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let form = reqwest::multipart::Form::new()
        .text("description", "not a file")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"hello world".to_vec()).file_name(file_name),
        );
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let content = tokio::fs::read(&path).await.unwrap();
    tokio::fs::remove_file(&path).await.unwrap();

    assert_eq!(upload_res.status(), StatusCode::OK);
    assert_eq!(content, b"hello world");
}

#[tokio::test]
async fn test_multipart_first_text_field_after_file_is_ignored() {
    dotenvy::dotenv().ok();

    let user_id = "multipart_first_text_after_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"hello world".to_vec()).file_name(file_name),
        )
        .text("description", "not a file");
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let content = tokio::fs::read(&path).await.unwrap();
    tokio::fs::remove_file(&path).await.unwrap();

    assert_eq!(upload_res.status(), StatusCode::OK);
    assert_eq!(content, b"hello world");
}

#[tokio::test]
async fn test_multipart_first_no_file_is_bad_request() {
    dotenvy::dotenv().ok();

    let user_id = "multipart_first_no_file_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    // Remove leftovers from previous failed runs, otherwise the existence check is meaningless
    let _ = tokio::fs::remove_file(&path).await;
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let form = reqwest::multipart::Form::new().text("description", "not a file");
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(upload_res.status(), StatusCode::BAD_REQUEST);
    assert!(!tokio::fs::try_exists(&path).await.unwrap());
}

fn over_limit_body() -> Vec<u8> {
    let max_body_limit: usize = std::env::var("MAX_BODY_LIMIT")
        .expect("MAX_BODY_LIMIT must be set")
        .parse()
        .unwrap();

    vec![0u8; (max_body_limit + 1) * 1024 * 1024]
}

// Atomic upload tests: data is written to a temporary file and renamed at the end.
// A failed upload must leave the previous file intact and no garbage behind.

#[tokio::test]
async fn test_multipart_upload_leaves_no_tmp_files() {
    dotenvy::dotenv().ok();

    let user_id = "multipart_atomic_no_tmp_user_id";
    let file_name = "my_file.txt";
    reset_user_dir(user_id).await;
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(b"hello world".to_vec()).file_name(file_name),
    );
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let entries = user_dir_entries(user_id).await;
    reset_user_dir(user_id).await;

    assert_eq!(upload_res.status(), StatusCode::OK);
    assert_eq!(entries, vec![file_name.to_string()]);
}

#[tokio::test]
async fn test_multipart_upload_overwrites_existing_file() {
    dotenvy::dotenv().ok();

    let user_id = "multipart_atomic_overwrite_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    reset_user_dir(user_id).await;
    create_file_and_dir(user_id, file_name).await;
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(b"new content".to_vec()).file_name(file_name),
    );
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let content = tokio::fs::read(&path).await.unwrap();
    let entries = user_dir_entries(user_id).await;
    reset_user_dir(user_id).await;

    assert_eq!(upload_res.status(), StatusCode::OK);
    assert_eq!(content, b"new content");
    assert_eq!(entries, vec![file_name.to_string()]);
}

#[tokio::test]
async fn test_multipart_upload_failure_keeps_existing_file() {
    dotenvy::dotenv().ok();

    let user_id = "multipart_atomic_keep_existing_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    reset_user_dir(user_id).await;
    create_file_and_dir(user_id, file_name).await;
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    // Over the body limit: the stream fails after some chunks were already written
    let client = Client::new();
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(over_limit_body()).file_name(file_name),
    );
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let content = tokio::fs::read(&path).await.unwrap();
    let entries = user_dir_entries(user_id).await;
    reset_user_dir(user_id).await;

    assert!(!upload_res.status().is_success());
    assert!(
        content == b"hello",
        "existing file was overwritten ({} bytes)",
        content.len()
    );
    assert_eq!(entries, vec![file_name.to_string()]);
}

#[tokio::test]
async fn test_multipart_upload_failure_leaves_no_garbage() {
    dotenvy::dotenv().ok();

    let user_id = "multipart_atomic_no_garbage_user_id";
    let file_name = "my_file.txt";
    reset_user_dir(user_id).await;
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(over_limit_body()).file_name(file_name),
    );
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let entries = user_dir_entries(user_id).await;
    reset_user_dir(user_id).await;

    assert!(!upload_res.status().is_success());
    assert_eq!(entries, Vec::<String>::new());
}

#[tokio::test]
async fn test_direct_upload_leaves_no_tmp_files() {
    dotenvy::dotenv().ok();

    let user_id = "direct_atomic_no_tmp_user_id";
    let file_name = "my_file.txt";
    reset_user_dir(user_id).await;
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .body(b"hello world".to_vec())
        .send()
        .await
        .unwrap();

    let entries = user_dir_entries(user_id).await;
    reset_user_dir(user_id).await;

    assert_eq!(upload_res.status(), StatusCode::OK);
    assert_eq!(entries, vec![file_name.to_string()]);
}

#[tokio::test]
async fn test_direct_upload_overwrites_existing_file() {
    dotenvy::dotenv().ok();

    let user_id = "direct_atomic_overwrite_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    reset_user_dir(user_id).await;
    create_file_and_dir(user_id, file_name).await;
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let upload_res = client
        .put(format!(
            "http://localhost:{port}/files/{user_id}/{file_name}?token={token}"
        ))
        .body(b"new content".to_vec())
        .send()
        .await
        .unwrap();

    let content = tokio::fs::read(&path).await.unwrap();
    let entries = user_dir_entries(user_id).await;
    reset_user_dir(user_id).await;

    assert_eq!(upload_res.status(), StatusCode::OK);
    assert_eq!(content, b"new content");
    assert_eq!(entries, vec![file_name.to_string()]);
}

#[tokio::test]
async fn test_direct_upload_failure_keeps_existing_file() {
    dotenvy::dotenv().ok();

    let user_id = "direct_atomic_keep_existing_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{user_id}/{file_name}");
    reset_user_dir(user_id).await;
    create_file_and_dir(user_id, file_name).await;
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let response = send_truncated_direct_upload(port, user_id, file_name, &token).await;

    let content = tokio::fs::read(&path).await.unwrap();
    let entries = user_dir_entries(user_id).await;
    reset_user_dir(user_id).await;

    assert!(!response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        content == b"hello",
        "existing file was overwritten ({} bytes)",
        content.len()
    );
    assert_eq!(entries, vec![file_name.to_string()]);
}

#[tokio::test]
async fn test_direct_upload_failure_leaves_no_garbage() {
    dotenvy::dotenv().ok();

    let user_id = "direct_atomic_no_garbage_user_id";
    let file_name = "my_file.txt";
    reset_user_dir(user_id).await;
    let token = generate_jwt_token(user_id, "upload", Some(file_name.to_string()));
    let port = spawn_serve().await.unwrap().port();

    let response = send_truncated_direct_upload(port, user_id, file_name, &token).await;

    let entries = user_dir_entries(user_id).await;
    reset_user_dir(user_id).await;

    assert!(!response.starts_with("HTTP/1.1 200"), "{response}");
    assert_eq!(entries, Vec::<String>::new());
}

/// Sends a PUT that promises more bytes than it delivers and then closes its write side,
/// so the body stream fails on the server after the first chunk was received.
/// Returns the raw HTTP response once the server has finished handling the request.
async fn send_truncated_direct_upload(
    port: u16,
    user_id: &str,
    file_name: &str,
    token: &str,
) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let head = format!(
        "PUT /files/{user_id}/{file_name}?token={token} HTTP/1.1\r\n\
         Host: localhost\r\n\
         Content-Length: 1000\r\n\
         Connection: close\r\n\r\n"
    );
    stream.write_all(head.as_bytes()).await.unwrap();
    stream.write_all(b"partial content").await.unwrap();
    stream.shutdown().await.unwrap();

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    String::from_utf8_lossy(&response).into_owned()
}

/// Sorted names of everything inside files/{user_id}, empty if the dir does not exist
async fn user_dir_entries(user_id: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let Ok(mut dir) = tokio::fs::read_dir(format!("files/{user_id}")).await else {
        return entries;
    };
    while let Some(entry) = dir.next_entry().await.unwrap() {
        entries.push(entry.file_name().to_string_lossy().into_owned());
    }
    entries.sort();
    entries
}

async fn reset_user_dir(user_id: &str) {
    let _ = tokio::fs::remove_dir_all(format!("files/{user_id}")).await;
}
