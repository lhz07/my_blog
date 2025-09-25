use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CatError {
    #[error("Toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Toml file error: {0}")]
    Walker(#[from] ignore::Error),
}

#[derive(Debug, Error)]
pub enum RespError {
    #[error("404 not found")]
    NotFound,
    #[error("500 server internal error")]
    InternalServerError,
}

impl From<CatError> for RespError {
    fn from(err: CatError) -> Self {
        match err {
            CatError::IO(_) | CatError::Toml(_) | CatError::Walker(_) => RespError::NotFound,
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
            Self::NotFound => HttpResponse::NotFound()
                .content_type("text/html")
                .body("<p>Not found!</p>"),
            Self::InternalServerError => HttpResponse::InternalServerError()
                .content_type("text/html")
                .body("<p>Something went wrong!</p>"),
        }
    }
}
