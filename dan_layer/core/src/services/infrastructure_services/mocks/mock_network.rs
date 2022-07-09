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

use std::sync::{Arc, RwLock};

use tokio::sync::broadcast::{self, Receiver, Sender};

use crate::{
    models::{HotStuffMessage, HotStuffMessageType, Payload, ViewId},
    services::infrastructure_services::{
        mocks::{MockInboundConnectionService, MockOutboundConnectionService},
        NodeAddressable,
    },
    DigitalAssetError,
};

#[derive(Clone, Debug)]
pub struct MockNetworkMessage<TAddr: NodeAddressable, TPayload: Payload> {
    from: TAddr,
    to: TAddr,
    message: HotStuffMessage<TPayload>,
}

#[derive(Clone)]
pub struct MockNetworkHandle<TAddr: NodeAddressable, TPayload: Payload> {
    inner: Arc<RwLock<MockNetworkInner<TAddr, TPayload>>>,
    new_message_event: Sender<()>,
}

impl<TAddr: NodeAddressable, TPayload: Payload> MockNetworkHandle<TAddr, TPayload> {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(10);
        Self {
            inner: Arc::new(RwLock::new(MockNetworkInner::new())),
            new_message_event: tx,
        }
    }

    pub fn create_inbound(&self, local_node_address: TAddr) -> MockInboundConnectionService<TAddr, TPayload> {
        MockInboundConnectionService {
            network: self.clone(),
            local_node_address,
        }
    }

    pub fn create_outbound(&self) -> MockOutboundConnectionService<TAddr, TPayload> {
        MockOutboundConnectionService::new(self.clone())
    }

    pub async fn wait_for_message(
        &self,
        to: &TAddr,
        message_type: HotStuffMessageType,
        for_view: ViewId,
    ) -> Result<(TAddr, HotStuffMessage<TPayload>), DigitalAssetError> {
        loop {
            let mut message = None;
            {
                let mut guard = self.inner.write().unwrap();
                message = guard.take_message(to, message_type, for_view)?;
            }
            todo!();

            if let Some(message) = message {
                return Ok((message.from, message.message));
            } else {
                // Wait for new messages, then try again...
                self.new_message_event.subscribe().recv().await.unwrap();
            }
        }
    }

    pub async fn send(
        &self,
        from: TAddr,
        to: TAddr,
        message: HotStuffMessage<TPayload>,
    ) -> Result<(), DigitalAssetError> {
        {
            let mut inner = self.inner.write().unwrap();
            inner.send(from, to, message);
        }

        self.new_message_event.send(());
        Ok(())
    }

    pub fn print_all_messages(&self) {
        let inner = self.inner.read().unwrap();
        inner.print_all_messages();
    }
}

pub struct MockNetworkInner<TAddr: NodeAddressable, TPayload: Payload> {
    all_messages: Vec<MockNetworkMessage<TAddr, TPayload>>,
    message_archive: Vec<MockNetworkMessage<TAddr, TPayload>>,
}

impl<TAddr: NodeAddressable, TPayload: Payload> MockNetworkInner<TAddr, TPayload> {
    pub fn new() -> Self {
        Self {
            all_messages: vec![],
            message_archive: vec![],
        }
    }

    pub fn take_message(
        &mut self,
        to: &TAddr,
        message_type: HotStuffMessageType,
        for_view: ViewId,
    ) -> Result<Option<MockNetworkMessage<TAddr, TPayload>>, DigitalAssetError> {
        let mut found_item = None;
        for (i, message) in self.all_messages.iter().enumerate() {
            if &message.to == to &&
                message.message.message_type() == message_type &&
                message.message.view_number() == for_view
            {
                found_item = Some((i, message.clone()));
                break;
            }
        }
        if let Some((i, message)) = found_item {
            self.all_messages.remove(i);
            return Ok(Some(message));
        } else {
            return Ok(None);
        }
    }

    pub fn send(
        &mut self,
        from: TAddr,
        to: TAddr,
        message: HotStuffMessage<TPayload>,
    ) -> Result<(), DigitalAssetError> {
        self.all_messages.push(MockNetworkMessage {
            from: from.clone(),
            to: to.clone(),
            message: message.clone(),
        });
        self.message_archive.push(MockNetworkMessage {
            from: from.clone(),
            to: to.clone(),
            message: message.clone(),
        });
        Ok(())
    }

    pub fn print_all_messages(&self) {
        for message in &self.message_archive {
            dbg!(message);
        }
    }
}
