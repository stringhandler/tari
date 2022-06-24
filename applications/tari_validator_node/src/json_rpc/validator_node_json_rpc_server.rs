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

use std::{collections::HashMap, net::SocketAddr};

use anyhow::Result;
use jsonrpsee::{http_server::HttpServerBuilder, RpcModule};
use serde::Deserialize;
use serde_json::Value;
use tari_common_types::types::PublicKey;
use tari_crypto::tari_utilities::hex::Hex;
use tari_dan_core::{models::AssetDefinition, services::ServiceSpecification, DigitalAssetError};
use tari_shutdown::ShutdownSignal;

pub struct ValidatorNodeJsonRpcServer<TServiceSpecification: ServiceSpecification> {
    asset_definitions: HashMap<PublicKey, AssetDefinition>,
    asset_processor: TServiceSpecification::AssetProcessor,
}

impl<TServiceSpecification: ServiceSpecification> ValidatorNodeJsonRpcServer<TServiceSpecification> {
    pub fn new(
        asset_definitions: HashMap<PublicKey, AssetDefinition>,
        asset_processor: TServiceSpecification::AssetProcessor,
    ) -> Self {
        Self {
            asset_definitions,
            asset_processor,
        }
    }

    pub async fn run(self, socket_addr: SocketAddr, shutdown_signal: ShutdownSignal) -> Result<()> {
        let server = HttpServerBuilder::default().build(socket_addr).await?;
        let mut module = RpcModule::new(());
        // for (key, value) in self.asset_definitions {
        //     for def in value.flow_functions {
        //         dbg!(key.to_hex());
        //         let k = key.to_hex();
        //         let mod_name = format!("{}_{}", &k, &def.name);
        //         module(&mod_name, |_, _| Ok("received"))?;
        //     }
        // }
        module.register_method("invoke", |params, context| {
            dbg!(context);

            let p: InvokeRequest = params.parse()?;
            dbg!(p);
            Ok("Invoked!")
        })?;
        module.register_method("say_hello", |_, _| Ok("lo"))?;

        let addr = server.local_addr()?;
        dbg!(addr);
        let _server_handle = server.start(module)?;
        shutdown_signal.await;
        // TODO: Shutdown
        Ok(())
    }
}

#[derive(Deserialize, Debug)]
struct InvokeRequest {
    asset: String,
    method: String,
    params: HashMap<String, Value>,
}
