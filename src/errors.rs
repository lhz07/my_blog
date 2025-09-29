use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;

use crate::not_found_page;

#[derive(Debug, Error)]
pub enum CatError {
    #[error("Toml parse error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("Toml parse error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Toml file error: {0}")]
    Walker(#[from] ignore::Error),
    #[error("Custom error: {0}")]
    Custom(String),
}

#[derive(Debug, Error)]
pub enum RespError {
    #[error("404 not found")]
    NotFound,
    #[error("500 server internal error")]
    InternalServerError,
    #[error("Custom error: {0}")]
    Custom(String),
}

impl From<CatError> for RespError {
    fn from(err: CatError) -> Self {
        match err {
            CatError::TomlDe(_) | CatError::Walker(_) => RespError::NotFound,
            CatError::IO(_) | CatError::TomlSer(_) => RespError::InternalServerError,
            CatError::Custom(s) => RespError::Custom(s),
        }
    }
}

impl From<tera::Error> for RespError {
    fn from(_err: tera::Error) -> Self {
        RespError::InternalServerError
    }
}

impl ResponseError for RespError {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        match self {
            Self::NotFound => not_found_page().unwrap_or(
                HttpResponse::NotFound()
                    .content_type("text/html")
                    .body("<p>Not found!</p>"),
            ),
            Self::InternalServerError => HttpResponse::InternalServerError()
                .content_type("text/html")
                .body("<p>Something went wrong!</p>"),
            Self::Custom(s) => HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body(s.clone()),
        }
    }
}
