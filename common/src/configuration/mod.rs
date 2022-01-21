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

//! # Configuration of tari applications
//!
//! Tari application consist of `common`, `base_node`, `wallet` and `application` configuration sections.
//! All tari apps follow traits implemented in this crate for ease and automation, for instance managing config files,
//! defaults configuration, overloading settings from subsections.
//!
//! ## Submodules
//!
//! - [bootstrap] - build CLI and manage/load configuration with [ConfigBootsrap] struct
//! - [global] - load GlobalConfig for Tari
//! - [loader] - build and load configuration modules in a tari-way
//! - [utils] - utilities for working with configuration
//!
//! ## Configuration file
//!
//! The tari configuration file (config.yml) is intended to be a single config file for all Tari desktop apps to use
//! to pull configuration variables, whether it's a testnet base node; wallet; validator node etc.
//!
//! The file lives in ~/.tari by default and has sections which will allow a specific app to determine
//! the config values it needs, e.g.
//!
//! ```toml
//! [common]
//! # Globally common variables
//! ...
//! [base_node]
//! # common vars for all base_node instances
//! [base_node.weatherwax]
//! # overrides for rincewnd testnet
//! [base_node.mainnet]
//! # overrides for mainnet
//! [wallet]
//! [wallet.weatherwax]
//! # etc..
//! ```

mod base_node_config;
mod bootstrap;
mod error;
mod global;
mod has_config_prefix;
mod loader;
mod merge_mining_config;
pub mod serialize;
mod tari_config;
mod utils;
mod validator_node_config;
mod wallet_config;
mod writer;

pub use error::ConfigurationError;
pub use tari_config::TariConfig;

// use base_node_config::BaseNodeConfig;
// use merge_mining_config::MergeMiningConfig;
// pub use validator_node_config::ValidatorNodeConfig;
// pub use wallet_config::WalletConfig;
