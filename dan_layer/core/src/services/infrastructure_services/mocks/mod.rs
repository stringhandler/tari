// Copyright 2021. The Tari Project
//
// Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
// following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
// disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
// following disclaimer in the documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
// products derived from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    RwLock,
};

use crate::{
    digital_assets_error::DigitalAssetError,
    models::HotStuffMessage,
    services::infrastructure_services::{InboundConnectionService, NodeAddressable, OutboundService},
};

pub mod mock_network;

// pub fn mock_inbound<TAddr: NodeAddressable, TPayload: Payload>() -> MockInboundConnectionService<TAddr, TPayload> {
//     MockInboundConnectionService::default()
// }

type Messages<TAddr, TPayload> = (
    Sender<(TAddr, HotStuffMessage<TPayload>)>,
    Receiver<(TAddr, HotStuffMessage<TPayload>)>,
);

#[derive()]
pub struct MockInboundConnectionService<TAddr: NodeAddressable, TPayload: Payload> {
    pub local_node_address: TAddr,
    pub network: MockNetworkHandle<TAddr, TPayload>, /* messages: Messages<TAddr, TPayload>,
                                                      * already_received_messages: Vec<(TAddr,
                                                      * HotStuffMessage<TPayload>)>, */
}

#[async_trait]
impl<TAddr: NodeAddressable + Send, TPayload: Payload> InboundConnectionService
    for MockInboundConnectionService<TAddr, TPayload>
{
    type Addr = TAddr;
    type Payload = TPayload;

    async fn wait_for_message(
        &self,
        message_type: HotStuffMessageType,
        for_view: ViewId,
    ) -> Result<(TAddr, HotStuffMessage<TPayload>), DigitalAssetError> {
        self.network
            .wait_for_message(&self.local_node_address, message_type, for_view)
            .await
    }

    async fn wait_for_qc(
        &self,
        _message_type: HotStuffMessageType,
        _for_view: ViewId,
    ) -> Result<(TAddr, HotStuffMessage<TPayload>), DigitalAssetError> {
        todo!()
    }
}

#[derive(Clone)]
pub struct MockOutboundConnectionService<TAddr: NodeAddressable, TPayload: Payload> {
    network: MockNetworkHandle<TAddr, TPayload>,
}

impl<TAddr: NodeAddressable, TPayload: Payload> MockOutboundConnectionService<TAddr, TPayload> {
    pub fn new(network: MockNetworkHandle<TAddr, TPayload>) -> Self {
        Self { network }
    }
}

use std::{fmt::Debug, sync::Arc};

use crate::{
    models::{HotStuffMessageType, Payload, ViewId},
    services::infrastructure_services::mocks::mock_network::MockNetworkHandle,
};

#[async_trait]
impl<TAddr: NodeAddressable + Send + Sync + Debug, TPayload: Payload> OutboundService
    for MockOutboundConnectionService<TAddr, TPayload>
{
    type Addr = TAddr;
    type Payload = TPayload;

    async fn send(
        &mut self,
        from: TAddr,
        to: TAddr,
        message: HotStuffMessage<TPayload>,
    ) -> Result<(), DigitalAssetError> {
        let t = &to;
        println!(
            "[mock] Sending message: {:?} {:?} sig:{:?}",
            &to,
            &message.message_type(),
            &message.partial_sig()
        );
        // intentionally swallow error here because the other end can die in tests
        // let _result = self.inbound_senders.get_mut(t).unwrap().send((from, message)).await;
        self.network.send(from, to, message).await?;
        Ok(())
    }

    async fn broadcast(
        &mut self,
        from: TAddr,
        committee: &[TAddr],
        message: HotStuffMessage<TPayload>,
    ) -> Result<(), DigitalAssetError> {
        for receiver in committee {
            self.send(from.clone(), receiver.clone(), message.clone()).await?
        }
        Ok(())
    }
}
