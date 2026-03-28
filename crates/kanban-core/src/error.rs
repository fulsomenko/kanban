use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("validation error: {0}")]
    Validation(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
