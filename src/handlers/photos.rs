use axum::{
    response::{ Response, IntoResponse },
    extract::{Path, Json, Multipart},
};
use crate::ResponseError;
use std::fs::File;
use std::io::Write;

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

        let name = field.name().unwrap().to_string(); // TODO Remove the unwrap
        let path = format!("files/{id}/{name}"); // TODO parse this to avoid inconsistencies and also handle errors when file already exists
        let mut file = create_file(&path, &name).map_err(|err| err)?;

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

pub fn create_file(path: &String, name: &String) -> Result<File, ResponseError> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|_err| ResponseError::Upload("Failed to create directory".to_string()))?;
    }

    // TODO wrong error handling
    Ok(File::create(&path).map_err(|_err| ResponseError::Upload("Failed to create file".to_string()))?)
}
