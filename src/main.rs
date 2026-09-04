use axum::{
    routing::get,
    Router,
};

#[tokio::main]
async fn main() {
   let app = Router::<()>::new()
       .route("/health", get(health));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn health() -> String {
    "igbbmn".to_string()
}
