use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
struct UploadClaims {
    sub: String,   // id del usuario
    exp: usize,    // expiración (unix timestamp)
    iat: usize,    // issued at (unix timestamp)
    scope: String, // Upload/Download
}

fn jwt_secret() -> String {
    std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set")
}

pub fn generate_jwt_upload_token(user_id: &str) -> String {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap().as_secs() as usize + 300; // 5 min
    let iat = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap().as_secs() as usize;
    let scope = "Upload".to_string();

    let claims = UploadClaims {
        sub: user_id.to_string(),
        iat,
        exp,
        scope,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    ).expect("failed to sign JWT")
}
