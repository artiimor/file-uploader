use axum::{
    response::{ Response, IntoResponse },
    extract::{Path, Multipart},
    http::{header, StatusCode},
    body::Body,
};
use tokio_util::io::ReaderStream;
use crate::ResponseError;
use std::fs::File;
use std::io::Write;
use tokio::fs::File as TokioFile;
use std::path::Path as StdPath;
use std::fs::remove_file;
use tokio::fs::metadata;

pub async fn health() -> Response {
    "igbbmn".to_string().into_response()
}

pub async fn get_files(Path((id, file_name)): Path<(String, String)>) -> Result<Response, ResponseError> {
    let path = format!("files/{id}/{file_name}"); // TODO parse name and id
    let ext = StdPath::new(&file_name)
        .extension()
        .ok_or(ResponseError::FileExtError)?; // TODO repasar manejo de errores en options y results
    let file = TokioFile::open(&path).await.map_err(|_| ResponseError::NotFound)?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let headers = [
        (header::CONTENT_TYPE, format!("application/{:?}", ext.to_str())), // TODO fix for all file type
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{file_name}\""),
        ),
    ];

    Ok((headers, body).into_response())
}

pub async fn upload_file(Path(id): Path<String>, mut multipart: Multipart) -> Result<Response, ResponseError> {
    while let Some(mut field) = multipart
            .next_field()
            .await
            .map_err(|err| ResponseError::Upload(err.to_string()))? { // TODO repasar esto, en especial el ?

        let name = field.name().unwrap().to_string(); // TODO Remove the unwrap
        let path = format!("files/{id}/{name}"); // TODO parse this to avoid inconsistencies and also handle errors when file already exists
        let mut file = create_file(&path).map_err(|err| err)?;

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

pub fn create_file(path: &String) -> Result<File, ResponseError> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|_err| ResponseError::Upload("Failed to create directory".to_string()))?;
    }

    // TODO wrong error handling
    Ok(File::create(&path).map_err(|_err| ResponseError::Upload("Failed to create file".to_string()))?)
}

pub async fn delete_file(Path((id, file_name)): Path<(String, String)>) -> Result<Response, ResponseError> {
    let file_path = format!("files/{id}/{file_name}");
    match remove_file(&file_path) {
        Ok(_) => Ok((StatusCode::OK, format!("file {file_path} deleted successfully!")).into_response()),
        Err(_) => Err(ResponseError::NotFound),
    }
}

pub async fn get_metadata(Path((id, file_name)): Path<(String, String)>) -> Result<Response, ResponseError> {
    let file_path = format!("files/{id}/{file_name}");
    let metadata = metadata(file_path).await.map_err(|_| ResponseError::NotFound)?;

    Ok((StatusCode::OK, format!("The file size is {} bytes!", metadata.len())).into_response())
}
