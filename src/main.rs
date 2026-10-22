use dotenvy::dotenv;

use axum_learning::app;

#[tokio::main]
async fn main() {
    dotenv().ok();
   let app = app();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3050").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

