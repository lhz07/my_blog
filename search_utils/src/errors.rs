use std::borrow::Cow;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Toml file error: {0}")]
    Walker(#[from] ignore::Error),
    #[error("Toml parse error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("Internal error: {0}")]
    Internal(Cow<'static, str>),
}

impl SearchError {
    pub fn internal<S: Into<Cow<'static, str>>>(s: S) -> Self {
        SearchError::Internal(s.into())
    }
}
