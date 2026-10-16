use std::net::SocketAddr;
use axum_learning::app;
use reqwest::{Client, StatusCode};

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
    let user_id = "test_user_id";
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

async fn spawn_serve() -> Result<SocketAddr, std::io::Error> {
    let app = app();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    let addr = listener.local_addr();

    tokio::spawn(async move { axum::serve(listener, app.clone()).await.unwrap() });

    addr
}
