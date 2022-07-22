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

use tari_comms::NodeIdentity;
use tari_dan_common_types::{Shard, ShardKey};
use tari_dan_engine::state::mocks::state_db::MockStateDbBackupAdapter;
use tokio::join;

use crate::{
    models::{domain_events::ConsensusWorkerDomainEvent, ConsensusHash, Payload},
    services::{
        infrastructure_services::mocks::{
            mock_network::MockNetworkHandle,
            MockInboundConnectionService,
            MockOutboundConnectionService,
        },
        mocks::{
            mock_shard_mapper::MockShardMapper,
            MockAssetProcessor,
            MockAssetProxy,
            MockBaseNodeClient,
            MockChainStorageService,
            MockCommitteeManager,
            MockEventsPublisher,
            MockMempoolService,
            MockPayloadProcessor,
            MockPayloadProvider,
            MockServiceSpecification,
            MockSigningService,
            MockStaticPayloadProvider,
            MockValidatorNodeClientFactory,
            MockWalletClient,
        },
        ConcreteAssetProxy,
        ConcreteCheckpointManager,
        ConcreteMempoolService,
        ServiceSpecification,
    },
    storage::mocks::{chain_db::MockChainDbBackupAdapter, MockDbFactory},
    workers::{single_payload_consensus_worker::SinglePayloadConsensusWorker, ConsensusWorkerBuilder},
};

fn create_identities_for_shards(num: usize) -> Vec<String> {
    let mut res = vec![];
    for i in 0..num {
        res.push(format!("shard{}", i));
    }
    res
}

fn create_workers_in_shards(
    num_shards: usize,
    network: &MockNetworkHandle<String, SimplePayload>,
) -> Vec<SinglePayloadConsensusWorker<MockServiceSpecification2>> {
    let identities = create_identities_for_shards(num_shards);

    let mut shard_mapper = MockShardMapper::new();
    let range_limit: u8 = 255 / num_shards as u8;
    for i in 0..num_shards - 1 {
        shard_mapper.assign(
            Shard { id: i as u64 },
            (i as u8 * range_limit)..((i as u8 + 1) * range_limit),
            identities[i].clone(),
        );
    }
    // last shard
    shard_mapper.assign(
        Shard {
            id: num_shards as u64 - 1,
        },
        (num_shards as u8 * range_limit)..255,
        identities[num_shards - 1].clone(),
    );

    let mut workers = vec![];
    for i in 0..num_shards {
        let worker = ConsensusWorkerBuilder::new()
            .with_network(network.clone())
            .with_shard_mapper(shard_mapper.clone())
            .with_identity(identities[i].clone())
            .build();
        workers.push(worker);
    }

    workers
}

#[derive(Debug, Clone)]
pub enum SimplePayload {
    Empty,
    StateChange {
        is_up: bool,
        data: Option<u8>,
        shard_key: ShardKey,
    },
}

impl Payload for SimplePayload {
    fn empty() -> Self {
        SimplePayload::Empty
    }

    fn involved_shard_keys(&self) -> Vec<ShardKey> {
        match self {
            SimplePayload::Empty => {
                vec![]
            },
            SimplePayload::StateChange { shard_key, .. } => {
                vec![shard_key.clone()]
            },
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            SimplePayload::Empty => true,
            SimplePayload::StateChange { .. } => false,
        }
    }
}
const empty_hash: [u8; 32] = [0; 32];
impl ConsensusHash for SimplePayload {
    fn consensus_hash(&self) -> &[u8] {
        match self {
            SimplePayload::Empty => &empty_hash,
            SimplePayload::StateChange { shard_key, .. } => shard_key.as_bytes(),
        }
    }
}

impl SimplePayload {
    pub fn bring_up(data: u8, shard_key: u8) -> Self {
        Self::StateChange {
            is_up: true,
            data: Some(data),
            shard_key: ShardKey::new(vec![shard_key]),
        }
    }

    pub fn bring_down(shard_key: u8) -> Self {
        Self::StateChange {
            is_up: false,
            data: None,
            shard_key: ShardKey::new(vec![shard_key]),
        }
    }
}

#[derive(Clone, Default)]
pub struct MockServiceSpecification2 {}

#[cfg(test)]
impl ServiceSpecification for MockServiceSpecification2 {
    type Addr = String;
    type AssetProcessor = MockAssetProcessor;
    type AssetProxy = MockAssetProxy;
    type BaseNodeClient = MockBaseNodeClient;
    type ChainDbBackendAdapter = MockChainDbBackupAdapter;
    type ChainStorageService = MockChainStorageService;
    type CheckpointManager = ConcreteCheckpointManager<Self::WalletClient>;
    type CommitteeManager = MockCommitteeManager<Self::Addr>;
    type DbFactory = MockDbFactory;
    type EventsPublisher = MockEventsPublisher<ConsensusWorkerDomainEvent>;
    type GlobalDbAdapter = crate::storage::mocks::global_db::MockGlobalDbBackupAdapter;
    type InboundConnectionService = MockInboundConnectionService<Self::Addr, Self::Payload>;
    type MempoolService = MockMempoolService;
    type OutboundService = MockOutboundConnectionService<Self::Addr, Self::Payload>;
    type Payload = SimplePayload;
    type PayloadProcessor = MockPayloadProcessor;
    type PayloadProvider = MockPayloadProvider<Self::Payload>;
    type SigningService = MockSigningService<Self::Addr>;
    type StateDbBackendAdapter = MockStateDbBackupAdapter;
    type ValidatorNodeClientFactory = MockValidatorNodeClientFactory<Self::Addr>;
    type WalletClient = MockWalletClient;
}

fn create_mock_network() -> MockNetworkHandle<String, SimplePayload> {
    MockNetworkHandle::new()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
pub async fn two_shards() {
    dbg!("hel");
    let network = create_mock_network();
    let mut workers = create_workers_in_shards(2, &network);
    dbg!("hello");
    let output1 = SimplePayload::bring_up(5, 0);
    let output2 = SimplePayload::bring_up(7, 1);
    workers[0].payload_provider_mut().push(output1);
    network.print_all_messages();

    let mut worker_one = workers.pop().unwrap();
    let mut worker_two = workers.pop().unwrap();
    let t1 = tokio::spawn(async move {
        worker_one.step().await.unwrap();
    });
    let t2 = tokio::spawn(async move {
        worker_two.step().await.unwrap();
    });
    let (res, res2) = join!(t1, t2);
    network.print_all_messages();
    res.expect("t1 failed");
    res2.expect("t2 failed");
    // mempool.add(tx);
    // workers[0].step();
    // workers[1].step();
}
