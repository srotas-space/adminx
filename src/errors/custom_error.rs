// crates/adminx/src/errors/custom_error.rs

use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;
use std::{fmt, error::Error};

#[derive(Debug, Serialize)]
pub struct ErrorResponseBody {
    pub code: u16,
    pub message: String,
    pub status: u16,
    pub data: String,
}

#[derive(Debug)]
pub enum CustomError {
    BadRequest(u16, String),
    InvalidRequest(u16, String),
    InternalError(u16, String),
    Unauthorized(u16, String),
    NotFound(u16, String),
    Conflict(u16, String),
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            CustomError::BadRequest(_, msg)
            | CustomError::InvalidRequest(_, msg)
            | CustomError::InternalError(_, msg)
            | CustomError::Unauthorized(_, msg)
            | CustomError::NotFound(_, msg)
            | CustomError::Conflict(_, msg) => msg,
        };
        write!(f, "{msg}")
    }
}

impl Error for CustomError {}

impl ResponseError for CustomError {
    fn status_code(&self) -> StatusCode {
        match self {
            CustomError::BadRequest(code, _)
            | CustomError::InvalidRequest(code, _)
            | CustomError::InternalError(code, _)
            | CustomError::Unauthorized(code, _)
            | CustomError::NotFound(code, _)
            | CustomError::Conflict(code, _) => {
                StatusCode::from_u16(*code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        let (code, msg) = match self {
            CustomError::BadRequest(code, msg)
            | CustomError::InvalidRequest(code, msg)
            | CustomError::InternalError(code, msg)
            | CustomError::Unauthorized(code, msg)
            | CustomError::NotFound(code, msg)
            | CustomError::Conflict(code, msg) => (*code, msg.clone()),
        };

        let body = ErrorResponseBody {
            code,
            message: msg.clone(),
            status: code,
            data: msg,
        };

        HttpResponse::build(self.status_code()).json(body)
    }
}


impl CustomError {
    pub async fn bad_request(code: u16, msg: impl Into<String> + Clone + std::fmt::Display) -> Self {
        CustomError::BadRequest(code, msg.into())
    }

    pub async fn invalid_request(code: u16, msg: impl Into<String> + Clone + std::fmt::Display) -> Self {
        CustomError::InvalidRequest(code, msg.into())
    }

    pub async fn internal_error(code: u16, msg: impl Into<String> + Clone + std::fmt::Display) -> Self {
        CustomError::InternalError(code, msg.into())
    }

    pub async fn unauthorized(code: u16, msg: impl Into<String> + Clone + std::fmt::Display) -> Self {
        CustomError::Unauthorized(code, msg.into())
    }

    pub async fn not_found(code: u16, msg: impl Into<String> + Clone + std::fmt::Display) -> Self {
        CustomError::NotFound(code, msg.into())
    }

    pub async fn conflict(code: u16, msg: impl Into<String> + Clone + std::fmt::Display) -> Self {
        CustomError::Conflict(code, msg.into())
    }
}
