use std::fmt;

#[derive(Debug)]
pub struct RunjucksError {
    message: String,
}

impl RunjucksError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RunjucksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RunjucksError {}

pub type Result<T> = std::result::Result<T, RunjucksError>;
