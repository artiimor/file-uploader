use std::net::SocketAddr;
use axum_learning::app;
use reqwest::{Client, StatusCode};

// TODO: test GET /upload_url/{user_id} as part of the upload flow above
// TODO: test GET /files/{id}/{file_name}/metadata (get_metadata) - currently has no auth at all
// TODO: test /download_url/{user_id}/{file_name} without Authorization header -> 401
// TODO same in DELETE
// TODO: test /files/{id}/{file_name} with missing/wrong-scope/wrong-sub token -> rejected
// TODO: test /files/{id}/{file_name} for a file that doesn't exist -> 404
// TODO: test /files/{id}/{file_name} with invalid id/file_name (e.g. path traversal) through the real router

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
async fn test_download_with_generated_url() {
    dotenvy::dotenv().ok();

    let bearer = std::env::var("API_KEY").unwrap();
    let user_id = "download_test_user_id";
    let file_name = "my_file.txt";

    let port = spawn_serve().await.unwrap().port();

    // If test fails is nevet deleted
    // TODO create temp dile with impl of drop that removes it always
    tokio::fs::create_dir_all(format!("files/{user_id}")).await.unwrap();
    tokio::fs::write(format!("files/{user_id}/{file_name}"), b"hello").await.unwrap();

    let client = Client::new();
    let res = client.get(format!("http://localhost:{port}/download_url/{user_id}/{file_name}"))
        .header("Authorization", format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().await.unwrap();
    let token = body.split("token=").nth(1).unwrap();

    let download_res = client.get(format!("http://localhost:{port}/files/{user_id}/{file_name}?token={token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(download_res.status(), StatusCode::OK);

    tokio::fs::remove_file(format!("files/{user_id}/{file_name}")).await.unwrap();
}

#[tokio::test]
async fn test_upload_with_generated_url() {
    dotenvy::dotenv().ok();

    let bearer = std::env::var("API_KEY").unwrap();
    let user_id = "upload_test_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{}/{}", user_id, file_name);
    let port = spawn_serve().await.unwrap().port();

    let client = Client::new();
    let res = client.get(format!("http://localhost:{port}/upload_url/{user_id}"))
        .header("Authorization", format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().await.unwrap();
    let token = body.split("token=").nth(1).unwrap();

    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(b"hello world".to_vec())
        .file_name(file_name));
    let upload_res = client.put(format!("http://localhost:{port}/files/{user_id}?token={token}"))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(upload_res.status(), StatusCode::OK);
    println!("{}", tokio::fs::try_exists(&path).await.unwrap());
    assert!(tokio::fs::try_exists(&path).await.unwrap());
    tokio::fs::remove_file(format!("files/{user_id}/{file_name}")).await.unwrap();
}

#[tokio::test]
async fn test_delete_file_with_bearer() {
    dotenvy::dotenv().ok();

    let bearer = std::env::var("API_KEY").unwrap();
    let user_id = "delete_test_user_id";
    let file_name = "my_file.txt";
    let path = format!("files/{}/{}", user_id, file_name);
    let port = spawn_serve().await.unwrap().port();
    create_file_and_dir(&user_id, &file_name).await;

    let client = Client::new();
    let delete_res = client.delete(format!("http://localhost:{port}/files/{user_id}/{file_name}"))
        .header("Authorization", format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap();
    
    assert_eq!(delete_res.status(), StatusCode::OK);
    assert!(!tokio::fs::try_exists(&path).await.unwrap());
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
