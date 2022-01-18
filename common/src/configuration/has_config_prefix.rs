//  Copyright 2021. The Tari Project
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

use config::Config;

use crate::ConfigurationError;

/// Load struct from config's main section and subsection override
pub trait HasConfigPrefix {
    /// Main configuration section
    fn main_key_prefix() -> &'static str;
    /// Overload values from a key prefix based on some configuration value.
    ///
    /// Should return a path to configuration table with overloading values.
    /// Returns `ConfigurationError` if key_prefix field has wrong value.
    /// Returns Ok(None) if no overload is required
    fn overload_key_prefix(config: &Config) -> Result<Option<String>, ConfigurationError>;
    /// Merge and produce sub-config from overload_key_prefix to main_key_prefix,
    /// which can be used to deserialize Self struct
    /// If overload key is not present in config it won't make effect
    fn merge_subconfig(config: &Config) -> Result<Config, ConfigurationError> {
        use config::Value;
        match Self::overload_key_prefix(config)? {
            Some(key) => {
                let overload: Value = config.get(key.as_str()).unwrap_or_default();
                let base: Value = config.get(Self::main_key_prefix()).unwrap_or_default();
                let mut base_config = Config::new();
                base_config.set(Self::main_key_prefix(), base)?;
                let mut config = Config::new();
                // Some magic is required to make them correctly merge
                config.merge(base_config)?;
                config.set(Self::main_key_prefix(), overload)?;
                Ok(config)
            },
            None => Ok(config.clone()),
        }
    }
}
