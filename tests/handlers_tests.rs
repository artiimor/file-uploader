use std::net::SocketAddr;
use file_uploader::app;
use reqwest::{Client, StatusCode};
use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, get_current_timestamp, Header, EncodingKey};

#[tokio::test]
async fn test_health_check_integration() {
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client.get(format!("http://0.0.0.0:{port}/health"))
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
    let download_res = client.get(format!("http://localhost:{port}/files/{user_id}/{file_name}?token={token}"))
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
    let download_res = client.get(format!("http://localhost:{port}/files/{user_id}/{file_name}?token={token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(download_res.status(), StatusCode::UNAUTHORIZED);

    tokio::fs::remove_file(format!("files/{user_id}/{file_name}")).await.unwrap();
    tokio::fs::remove_file(format!("files/{user_id}/{other_file_name}")).await.unwrap();
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
    let download_res = client.get(format!("http://localhost:{port}/files/{user_id}/{file_name}?token={token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(download_res.status(), StatusCode::UNAUTHORIZED);

    tokio::fs::remove_file(format!("files/{user_id}/{file_name}")).await.unwrap();
    tokio::fs::remove_file(format!("files/{other_user_id}/{file_name}")).await.unwrap();
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
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(b"hello world".to_vec())
        .file_name(file_name));
    let upload_res = client.put(format!("http://localhost:{port}/files/{user_id}/{file_name}?token={token}"))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(upload_res.status(), StatusCode::OK);
    assert!(tokio::fs::try_exists(&path).await.unwrap());
    assert_eq!(tokio::fs::read(&path).await.unwrap(), b"hello world");
    tokio::fs::remove_file(format!("files/{user_id}/{file_name}")).await.unwrap();
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
    let upload_res = client.put(format!("http://localhost:{port}/files/{user_id}/{file_name}?token={token}"))
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
    let delete_res = client.delete(format!("http://localhost:{port}/files/{user_id}/{file_name}?token={token}"))
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
        let delete_res = client.delete(format!("http://localhost:{port}/files/{user_id}/{file_name}?token={token}"))
            .send()
            .await
            .unwrap();

        assert_eq!(delete_res.status(), StatusCode::UNAUTHORIZED);
    }

    assert!(tokio::fs::try_exists(&path).await.unwrap());
}

#[tokio::test]
async fn test_get_metadata() {
    let user_id = "metadata_test_user_id";
    let file_name = "my_file.txt";

    create_file_and_dir(user_id, file_name).await;

    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client.get(format!("http://localhost:{port}/files/{user_id}/{file_name}/metadata"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().await.unwrap();
    assert_eq!(body, "The file size is 5 bytes!");

    tokio::fs::remove_file(format!("files/{user_id}/{file_name}")).await.unwrap();
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
    tokio::fs::create_dir_all(format!("files/{user_id}")).await.unwrap();
    tokio::fs::write(format!("files/{user_id}/{file_name}"), b"hello").await.unwrap();
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,   // user id
    exp: u64,
    scope: String,
    file_name: Option<String>,
}

fn generate_jwt_token(user_id: &str, scope: &str, file_name: Option<String>) -> String {
    let token_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");

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
    ).unwrap()
}

