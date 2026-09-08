use axum::{
    response::{ Response, IntoResponse },
    extract::{Path, Json, Multipart},
};
use serde::Deserialize;
use crate::ResponseError;
use std::fs;
use std::fs::File;
use std::io::Write;

#[derive(Deserialize)]
pub struct MyPayload {
    file_path: String,
}

pub async fn health() -> Response {
    "igbbmn".to_string().into_response()
}

pub async fn get_files(Path(photo_path): Path<String>) -> String {
    // TODO download or error
    photo_path
}

pub async fn upload_file(Path(id): Path<String>, mut multipart: Multipart) -> Result<Response, ResponseError> {
    // TODO Upload file
    while let Some(mut field) = multipart
            .next_field()
            .await
            .map_err(|err| ResponseError::Upload(err.to_string()))? { // TODO repasar esto, en especial el ?

         // TODO extract function for create the file and directories
        let name = field.name().unwrap().to_string();
        let path = format!("files/{id}/{name}"); // TODO parse this to avoid inconsistencies and also handle errors when file already exists
        // Create parent directories
        if let Some(parent) = std::path::Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|_err| ResponseError::Upload("Failed to create directory".to_string()))?;
        }
        // Now write the file
        let mut file = File::create(&path).map_err(|_err| ResponseError::Upload("Failed to create file".to_string()))?;

        // Write the data
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|err| ResponseError::Upload(err.to_string()))?
        {

            file.write_all(&chunk)
                .map_err(|err| ResponseError::Upload(err.to_string()))?;
        }
    }
    Ok("File uploaded!".to_string().into_response())
}
