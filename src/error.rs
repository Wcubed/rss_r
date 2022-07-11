use actix_web::{http::StatusCode, HttpResponse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Authentication failure")]
    AuthenticationError,
}

impl actix_web::error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Error::AuthenticationError => StatusCode::UNAUTHORIZED,
        }
    }
}
