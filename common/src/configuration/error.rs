//  Copyright 2022. The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::{
    error::Error,
    fmt,
    fmt::{Display, Formatter},
};

use structopt::clap::Error as ClapError;

#[derive(Debug)]
pub struct ConfigurationError {
    field: String,
    value: Option<String>,
    message: String,
}

impl ConfigurationError {
    pub fn new<F: Into<String>, M: Into<String>>(field: F, value: Option<String>, msg: M) -> Self {
        ConfigurationError {
            field: field.into(),
            value: value.map(|s| s),
            message: msg.into(),
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
                value: None,
                message: cause.to_string(),
            },
            ConfigError::Type {
                ref unexpected,
                ref key,
                ..
            } => Self {
                field: format!("{:?}", key),
                value: Some(unexpected.to_string()),
                message: err.to_string(),
            },
            ConfigError::NotFound(key) => Self {
                field: key,
                value: None,
                message: "required key not found".to_string(),
            },
            x => Self::new("", None, x.to_string()),
        }
    }
}
impl From<serde_json::error::Error> for ConfigurationError {
    fn from(err: serde_json::error::Error) -> Self {
        Self {
            field: "".to_string(),
            value: None,
            message: err.to_string(),
        }
    }
}
