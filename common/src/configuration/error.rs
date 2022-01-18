use std::{
    error::Error,
    fmt,
    fmt::{Display, Formatter},
};

use structopt::clap::Error as ClapError;

#[derive(Debug)]
pub struct ConfigurationError {
    field: String,
    message: String,
}

impl ConfigurationError {
    pub fn new(field: &str, msg: &str) -> Self {
        ConfigurationError {
            field: String::from(field),
            message: String::from(msg),
        }
    }
}

impl Display for ConfigurationError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Invalid value for {}: {}", self.field, self.message)
    }
}

impl Error for ConfigurationError {}

impl From<config::ConfigError> for ConfigurationError {
    fn from(err: config::ConfigError) -> Self {
        use config::ConfigError;
        match err {
            ConfigError::FileParse { uri, cause } if uri.is_some() => Self {
                field: uri.unwrap(),
                message: cause.to_string(),
            },
            ConfigError::Type { ref key, .. } => Self {
                field: format!("{:?}", key),
                message: err.to_string(),
            },
            ConfigError::NotFound(key) => Self {
                field: key,
                message: "required key not found".to_string(),
            },
            x => Self::new("", x.to_string().as_str()),
        }
    }
}
impl From<serde_json::error::Error> for ConfigurationError {
    fn from(err: serde_json::error::Error) -> Self {
        Self {
            field: "".to_string(),
            message: err.to_string(),
        }
    }
}
