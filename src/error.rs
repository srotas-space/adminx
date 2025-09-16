// crates/adminx/src/error.rs

use actix_web::{HttpResponse, ResponseError};
use derive_more::Display;
use serde::Serialize;

#[derive(Debug, Display)]
pub enum AdminxError {
    #[display(fmt = "Not Found")]
    NotFound,
    #[display(fmt = "Bad Request: {}", _0)]
    BadRequest(String),
    #[display(fmt = "Internal Server Error")]
    InternalError,
}

impl std::error::Error for AdminxError {}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl ResponseError for AdminxError {
    fn error_response(&self) -> HttpResponse {
        let status = match self {
            AdminxError::NotFound => actix_web::http::StatusCode::NOT_FOUND,
            AdminxError::BadRequest(_) => actix_web::http::StatusCode::BAD_REQUEST,
            AdminxError::InternalError => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        };

        HttpResponse::build(status).json(ErrorResponse {
            error: self.to_string(),
        })
    }
}
