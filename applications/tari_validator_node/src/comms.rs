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

use std::sync::Arc;

use log::*;
use tari_app_utilities::{
    identity_management,
    identity_management::load_from_json,
    utilities::convert_socks_authentication,
};
use tari_common::exit_codes::ExitCodes;
use tari_comms::{
    protocol::rpc::RpcServer,
    socks,
    tor,
    tor::TorIdentity,
    transports::{predicate::FalsePredicate, SocksConfig},
    utils::multiaddr::multiaddr_to_socketaddr,
    NodeIdentity,
    UnspawnedCommsNode,
};
use tari_comms_dht::{store_forward::SafConfig, DbConnectionUrl, Dht, DhtConfig};
use tari_dan_core::services::{ConcreteAssetProcessor, MempoolServiceHandle};
use tari_dan_storage_sqlite::SqliteDbFactory;
use tari_p2p::{
    comms_connector::{pubsub_connector, SubscriptionFactory},
    initialization::{spawn_comms_using_transport, P2pConfig, P2pInitializer},
    transport::{TorConfig, TransportType},
};
use tari_service_framework::{ServiceHandles, StackBuilder};
use tari_shutdown::ShutdownSignal;

use crate::p2p::create_validator_node_rpc_service;

const LOG_TARGET: &str = "tari::validator_node::comms";

pub async fn build_service_and_comms_stack(
    p2p_config: &P2pConfig,
    shutdown: ShutdownSignal,
    node_identity: Arc<NodeIdentity>,
    mempool: MempoolServiceHandle,
    db_factory: SqliteDbFactory,
    asset_processor: ConcreteAssetProcessor,
) -> Result<(ServiceHandles, SubscriptionFactory), ExitCodes> {
    let (publisher, peer_message_subscriptions) = pubsub_connector(100, p2p_config.inbound_rate_limit);

    let mut handles = StackBuilder::new(shutdown.clone())
        .add_initializer(P2pInitializer::new(p2p_config.clone(), publisher))
        .build()
        .await
        .map_err(|err| ExitCodes::ConfigError(err.to_string()))?;

    let comms = handles
        .take_handle::<UnspawnedCommsNode>()
        .expect("P2pInitializer was not added to the stack or did not add UnspawnedCommsNode");

    let comms = setup_p2p_rpc(config, comms, &handles, mempool, db_factory, asset_processor);

    let comms = spawn_comms_using_transport(comms, create_transport_type(config))
        .await
        .map_err(|e| ExitCodes::ConfigError(format!("Could not spawn using transport:{}", e)))?;

    // Save final node identity after comms has initialized. This is required because the public_address can be
    // changed by comms during initialization when using tor.
    identity_management::save_as_json(&config.base_node_identity_file, &*comms.node_identity())
        .map_err(|e| ExitCodes::ConfigError(format!("Failed to save node identity: {}", e)))?;
    if let Some(hs) = comms.hidden_service() {
        identity_management::save_as_json(&config.base_node_tor_identity_file, hs.tor_identity())
            .map_err(|e| ExitCodes::ConfigError(format!("Failed to save tor identity: {}", e)))?;
    }

    handles.register(comms);
    Ok((handles, peer_message_subscriptions))
}

fn setup_p2p_rpc(
    config: &GlobalConfig,
    comms: UnspawnedCommsNode,
    handles: &ServiceHandles,
    mempool: MempoolServiceHandle,
    db_factory: SqliteDbFactory,
    asset_processor: ConcreteAssetProcessor,
) -> UnspawnedCommsNode {
    let dht = handles.expect_handle::<Dht>();
    let builder = RpcServer::builder();
    let builder = match config.rpc_max_simultaneous_sessions {
        Some(limit) => builder.with_maximum_simultaneous_sessions(limit),
        None => {
            warn!(
                target: LOG_TARGET,
                "Node is configured to allow unlimited RPC sessions."
            );
            builder.with_unlimited_simultaneous_sessions()
        },
    };
    let rpc_server = builder.finish();

    // Add your RPC services here ‍🏴‍☠️️☮️🌊
    let rpc_server = rpc_server
        .add_service(dht.rpc_service())
        .add_service(create_validator_node_rpc_service(mempool, db_factory, asset_processor));

    comms.add_protocol_extension(rpc_server)
}
