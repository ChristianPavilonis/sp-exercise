use axum::{body::Body, http::{Response, StatusCode}, response::IntoResponse};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CustomError>;

#[derive(Debug, Error)]
pub enum CustomError {
    #[error("Record not found")]
    RecordNotFound,
    #[error("Something went wrong!")]
    Other(#[from] anyhow::Error),
}


impl IntoResponse for CustomError {
    fn into_response(self) -> Response<Body> {
        let (status, message) = match self {
            CustomError::RecordNotFound => (StatusCode::NOT_FOUND, "404 Record not found"),
            CustomError::Other(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong!"),
        };

        let body = Body::from(message.to_string());

        Response::builder().status(status).body(body).unwrap()
    }
}
