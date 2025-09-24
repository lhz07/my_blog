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
