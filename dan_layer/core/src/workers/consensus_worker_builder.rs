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

use tokio::time::Duration;

use crate::{
    services::{
        infrastructure_services::mocks::mock_network::MockNetworkHandle,
        mocks::{
            mock_base_node_client,
            mock_checkpoint_manager,
            mock_committee_manager,
            mock_events_publisher,
            mock_payload_processor,
            mock_payload_provider,
            mock_shard_mapper::MockShardMapper,
            mock_signing_service,
            mock_validator_node_client_factory,
            MockChainStorageService,
        },
        ServiceSpecification,
    },
    storage::mocks::MockDbFactory,
    workers::{tests::consensus_worker_tests::MockServiceSpecification2, ConsensusWorker},
};

#[cfg(test)]
pub struct ConsensusWorkerBuilder<TSpecification: ServiceSpecification> {
    identity: Option<TSpecification::Addr>,
    shard_mapper: Option<MockShardMapper<TSpecification::Addr>>,
    phantom: std::marker::PhantomData<TSpecification>,
    network: Option<MockNetworkHandle<TSpecification::Addr, TSpecification::Payload>>,
}

impl<TSpecification: ServiceSpecification> ConsensusWorkerBuilder<TSpecification> {
    pub fn new() -> Self {
        Self {
            identity: None,
            network: None,
            shard_mapper: None,
            phantom: Default::default(),
        }
    }

    pub fn with_identity(mut self, addr: TSpecification::Addr) -> Self {
        self.identity = Some(addr);
        self
    }

    pub fn with_network(mut self, network: MockNetworkHandle<TSpecification::Addr, TSpecification::Payload>) -> Self {
        self.network = Some(network);
        self
    }

    pub fn with_shard_mapper(mut self, shard_mapper: MockShardMapper<TSpecification::Addr>) -> Self {
        self.shard_mapper = Some(shard_mapper);
        self
    }
}

impl ConsensusWorkerBuilder<MockServiceSpecification2> {
    pub fn build(self) -> ConsensusWorker<MockServiceSpecification2> {
        let identity = self.identity.expect("Must have an identity");
        let network = self.network.expect("Network must be provided");
        let shard_mapper = self.shard_mapper.expect("Shard mapper must be provided");
        let shards = shard_mapper.get_shards();
        let current_shard = shard_mapper
            .find_shard_for(&identity)
            .expect("Identity must be mapped to a shard");
        ConsensusWorker::new(
            network.create_inbound(identity.clone()),
            network.create_outbound(),
            mock_committee_manager(shard_mapper, current_shard),
            identity.clone(),
            mock_payload_provider(),
            mock_events_publisher(),
            mock_signing_service(),
            mock_payload_processor(),
            Default::default(),
            mock_base_node_client(),
            Duration::from_secs(5),
            MockDbFactory::default(),
            MockChainStorageService::default(),
            mock_checkpoint_manager(),
            mock_validator_node_client_factory(),
        )
    }
}
