use axum::http::StatusCode;
use reqwest::Client;
use std::net::SocketAddr;
use axum_learning::app;

#[tokio::test]
async fn test_health_check_integration() {
    let app = app();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3099").await.unwrap();

    tokio::spawn(async move { axum::serve(listener, app.clone()).await.unwrap() });

    let client = Client::new();
    let res = client.get("http://localhost:3099/health")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().await.unwrap();
    assert_eq!(body, "igbbmn");
}
