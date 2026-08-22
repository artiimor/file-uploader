use jsonwebtoken::{encode, EncodingKey, Header, decode, DecodingKey, Validation, TokenData, Algorithm};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use axum::http::StatusCode;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    sub: String,   // id del usuario
    exp: usize,    // expiración (unix timestamp)
    iat: usize,    // issued at (unix timestamp)
    scope: String, // Upload/Download
}

fn jwt_secret() -> String {
    std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set")
}

fn validation() -> Validation {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 0;
    validation
}

pub fn generate_jwt_upload_token(user_id: &str) -> String {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap().as_secs() as usize + 300; // 5 min
    let iat = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap().as_secs() as usize;
    let scope = "Upload".to_string();

    let claims = Claims {
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

// By default it usus HS256 which is a symetric algorithm
pub fn decode_jwt_token(token: &str) -> Result<TokenData<Claims>, StatusCode> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_ref()),
        &validation(), // Actually valiudates exp, nbf, aud, iss, sub
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(token_data)
}
