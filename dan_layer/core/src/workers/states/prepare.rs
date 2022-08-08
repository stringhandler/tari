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

use std::{collections::HashMap, time::Duration};

use log::*;
use tari_common_types::types::FixedHash;
use tari_dan_common_types::Shard;
use tari_dan_engine::state::StateDbUnitOfWork;
use tokio::time::sleep;

use crate::{
    digital_assets_error::DigitalAssetError,
    models::{
        AssetDefinition,
        Committee,
        HotStuffMessage,
        HotStuffMessageType,
        HotStuffTreeNode,
        MergedVoteBuilder,
        Payload,
        QuorumCertificate,
        TreeNodeHash,
        View,
        ViewId,
    },
    services::{
        infrastructure_services::{InboundConnectionService, OutboundService},
        CommitteeManager,
        PayloadProcessor,
        PayloadProvider,
        ServiceSpecification,
        SigningService,
    },
    storage::{chain::ChainDbUnitOfWork, ChainStorageService, DbFactory, StorageError},
    workers::states::ConsensusWorkerStateEvent,
};

const LOG_TARGET: &str = "tari::dan::workers::states::prepare";

pub struct Prepare<TSpecification: ServiceSpecification> {
    node_id: TSpecification::Addr,
    shard: Shard,
    received_new_view_messages: HashMap<TSpecification::Addr, HotStuffMessage<TSpecification::Payload>>,
    merged_vote_builder: MergedVoteBuilder,
}

impl<TSpecification: ServiceSpecification> Prepare<TSpecification> {
    pub fn new(node_id: TSpecification::Addr, shard: Shard) -> Self {
        Self {
            node_id,
            shard,
            received_new_view_messages: HashMap::new(),
            merged_vote_builder: MergedVoteBuilder::new(),
        }
    }

    pub async fn next_event<
        TStateDbUnitOfWork: StateDbUnitOfWork,
        TPayloadProvider: PayloadProvider<TSpecification::Payload>,
    >(
        &mut self,
        current_view: &View,
        timeout: Duration,
        committee: &Committee<TSpecification::Addr>,
        committee_manager: &TSpecification::CommitteeManager,
        inbound_services: &TSpecification::InboundConnectionService,
        outbound_service: &mut TSpecification::OutboundService,
        payload_provider: &TPayloadProvider,
        signing_service: &TSpecification::SigningService,
        payload_processor: &mut TSpecification::PayloadProcessor,
        // mut chain_tx: TChainDbUnitOfWork,
        state_tx: &mut TStateDbUnitOfWork,
        db_factory: &TSpecification::DbFactory,
    ) -> Result<ConsensusWorkerStateEvent, DigitalAssetError> {
        self.received_new_view_messages.clear();
        let timeout = sleep(timeout);
        futures::pin_mut!(timeout);
        debug!(target: LOG_TARGET, "[PREPARE] Current view: {}", current_view);

        if current_view.is_leader() {
            debug!(
                target: LOG_TARGET,
                "Waiting for NewView (view_id = {}) messages",
                current_view.view_id()
            );
        } else {
            debug!(
                target: LOG_TARGET,
                "Waiting for Prepare (view_id = {}) messages",
                current_view.view_id()
            );
        }
        loop {
            tokio::select! {
                r = inbound_services.wait_for_message(HotStuffMessageType::NewView, current_view.view_id())  => {
                    let (from, message) = r?;
                    debug!(target: LOG_TARGET, "Received leader message (is_leader = {:?})", current_view.is_leader());
                    dbg!(&from);
                    dbg!(&message);
                    if current_view.is_leader() {
                        if let Some(event) = self.process_leader_message(
                            current_view,
                            message.clone(),
                            &from,
                            committee,
                            committee_manager,
                            payload_provider,
                            payload_processor,
                            outbound_service,
                            db_factory,
                        ).await? {
                            break Ok(event)
                        }
                    }
                },
                r = inbound_services.wait_for_message(HotStuffMessageType::Prepare, current_view.view_id()) => {
                    let (from, message) = r?;
                    debug!(target: LOG_TARGET, "Received replica message");
                    if let Some(event) = self.process_replica_message(
                        &message,
                        current_view,
                        &from,
                        committee.leader_for_view(current_view.view_id),
                        outbound_service,
                        signing_service,
                        payload_processor,
                        // &mut chain_tx,
                        committee_manager,
                        state_tx,
                    ).await? {
                        break Ok(event);
                    }

                },
                _ = &mut timeout => {
                    todo!();
                    break Ok( ConsensusWorkerStateEvent::TimedOut);
                }
            }
        }
    }

    async fn process_leader_message<TPayloadProvider: PayloadProvider<TSpecification::Payload>>(
        &mut self,
        current_view: &View,
        message: HotStuffMessage<TSpecification::Payload>,
        sender: &TSpecification::Addr,
        previous_committee: &Committee<TSpecification::Addr>,
        committee_manager: &TSpecification::CommitteeManager,
        payload_provider: &TPayloadProvider,
        payload_processor: &mut TSpecification::PayloadProcessor,
        outbound: &mut TSpecification::OutboundService,
        db_factory: &TSpecification::DbFactory,
    ) -> Result<Option<ConsensusWorkerStateEvent>, DigitalAssetError> {
        debug!(
            target: LOG_TARGET,
            "Received message as leader:{:?} for view:{}",
            message.message_type(),
            message.view_number()
        );

        // TODO: This might need to be checked in the QC rather
        if self.received_new_view_messages.contains_key(sender) {
            println!("Already received message from {:?}", sender);
            return Ok(None);
        }

        if !committee_manager.current_committee()?.contains(sender) {
            warn!(target: LOG_TARGET, "Received message from non-member: {:?}", sender);
            return Ok(None);
        }

        self.received_new_view_messages.insert(sender.clone(), message);

        if self.received_new_view_messages.len() >= previous_committee.consensus_threshold() {
            debug!(
                target: LOG_TARGET,
                "[PREPARE] Consensus has been reached with {:?} out of {} votes",
                self.received_new_view_messages.len(),
                previous_committee.len()
            );
            let high_qc = self.find_highest_qc();

            let temp_state_tx = db_factory
                .get_or_create_state_db()?
                .new_unit_of_work(current_view.view_id.as_u64());
            let parent = high_qc.node_hashes().get(&self.shard).unwrap();
            todo!("proposal");
            // let proposal = self
            //     .create_proposal(
            //         parent.clone(),
            //         asset_definition,
            //         payload_provider,
            //         payload_processor,
            //         current_view.view_id,
            //         temp_state_tx,
            //     )
            //     .await?;
            // let shard_keys = proposal.payload().involved_shard_keys();
            // dbg!(&shard_keys);
            // let total_committee_set = committee_manager.get_node_set_for_shards(&shard_keys)?;
            // dbg!(&total_committee_set);
            // self.broadcast_proposal(outbound, &total_committee_set, proposal, high_qc, current_view.view_id)
            //     .await?;
            Ok(None) // Will move to pre-commit when it receives the message as a replica
        } else {
            debug!(
                target: LOG_TARGET,
                "[PREPARE] Consensus has NOT YET been reached with {} out of {} votes",
                self.received_new_view_messages.len(),
                previous_committee.len()
            );
            Ok(None)
        }
    }

    async fn process_replica_message<TStateDbUnitOfWork: StateDbUnitOfWork>(
        &mut self,
        message: &HotStuffMessage<TSpecification::Payload>,
        current_view: &View,
        from: &TSpecification::Addr,
        view_leader: &TSpecification::Addr,
        outbound: &mut TSpecification::OutboundService,
        signing_service: &TSpecification::SigningService,
        payload_processor: &mut TSpecification::PayloadProcessor,
        committee_manager: &TSpecification::CommitteeManager,
        state_tx: &mut TStateDbUnitOfWork,
    ) -> Result<Option<ConsensusWorkerStateEvent>, DigitalAssetError> {
        debug!(
            target: LOG_TARGET,
            "Received message as replica:{:?} for view:{}",
            message.message_type(),
            message.view_number()
        );
        if message.node().is_none() {
            unimplemented!("Empty message");
        }
        let node = message.node().unwrap();

        // TODO: Maybe process empty payloads
        if node.payload().is_empty() {
            dbg!("Empty payload");
            return Ok(None);
        }

        let justify = message
            .justify()
            .ok_or(DigitalAssetError::PreparePhaseNoQuorumCertificate)?;
        dbg!(&node.payload().involved_shard_keys());
        let involved_shards = committee_manager.get_shards_for_keys(&node.payload().involved_shard_keys())?;
        let involved_nodes = committee_manager.get_node_set_for_shards(&node.payload().involved_shard_keys())?;
        if !involved_shards.contains_key(&message.shard()) {
            return Err(DigitalAssetError::MessageReceivedForWrongShard);
        }

        if !involved_shards.contains_key(&self.shard) {
            return Err(DigitalAssetError::MessageReceivedForWrongShard);
        }

        if (message.shard() == self.shard && from != view_leader) ||
            !committee_manager
                .get_shard_committee(message.shard())
                .unwrap()
                .contains(&from)
        {
            println!("Message not from leader");
            return Ok(None);
        }

        if message.shard() == self.shard {
            if !self.does_extend(node, justify.node_hash(committee_manager.current_shard()?).unwrap()) {
                return Err(DigitalAssetError::PreparePhaseCertificateDoesNotExtendNode);
            }

            todo!("safe node needs to be fixed");

            // if !self.is_safe_node(node, justify, chain_tx)? {
            //     return Err(DigitalAssetError::PreparePhaseNodeNotSafe);
            // }

            debug!(
                target: LOG_TARGET,
                "[PREPARE] Processing prepared payload for view {}",
                current_view.view_id()
            );

            let state_root = payload_processor
                .process_payload(node.payload(), state_tx.clone())
                .await?;

            if state_root != *node.state_root() {
                warn!(
                    target: LOG_TARGET,
                    "Calculated state root did not match the state root provided by the leader: Expected: {:?} Leader \
                     provided:{:?}",
                    state_root,
                    node.state_root()
                );
                return Ok(None);
            }

            debug!(
                target: LOG_TARGET,
                "[PREPARE] Merkle root matches payload for view {}. Adding node '{}'",
                current_view.view_id(),
                node.hash()
            );

            todo!("Add node needs to be fixed");
            // chain_storage_service
            //     .add_node::<TChainDbUnitOfWork>(node, chain_tx.clone())
            //     .await?;
        }

        self.merged_vote_builder.add(message.shard(), node.hash().clone());

        if self
            .merged_vote_builder
            .is_complete(involved_shards.keys().map(|k| *k).collect::<Vec<_>>().as_slice())
        {
            let nodes = self.merged_vote_builder.build_and_clear();
            // todo!("Do we need to reserve the payload?");
            // payload_provider.reserve_payload(node.payload(), node.hash()).await?;
            self.send_vote_to_leader(nodes, outbound, view_leader, current_view.view_id, signing_service)
                .await?;
            Ok(Some(ConsensusWorkerStateEvent::Prepared))
        } else {
            Ok(None)
        }
    }

    fn find_highest_qc(&self) -> QuorumCertificate {
        let mut max_qc = None;
        for message in self.received_new_view_messages.values() {
            match &max_qc {
                None => max_qc = message.justify().cloned(),
                Some(qc) => {
                    if let Some(justify) = message.justify() {
                        if qc.view_number() < justify.view_number() {
                            max_qc = Some(justify.clone())
                        }
                    }
                },
            }
        }
        // TODO: this will panic if nothing found
        max_qc.unwrap()
    }

    #[allow(clippy::cast_possible_truncation)]
    async fn create_proposal<TStateDbUnitOfWork: StateDbUnitOfWork>(
        &self,
        parent: TreeNodeHash,
        asset_definition: &AssetDefinition,
        payload_provider: &TSpecification::PayloadProvider,
        payload_processor: &mut TSpecification::PayloadProcessor,
        view_id: ViewId,
        state_db: TStateDbUnitOfWork,
    ) -> Result<HotStuffTreeNode<TSpecification::Payload>, DigitalAssetError> {
        debug!(target: LOG_TARGET, "Creating new proposal for {}", view_id);

        // if view_id.is_genesis() {
        //     let payload = payload_provider.create_genesis_payload(asset_definition);
        //     let state_root = payload_processor.process_payload(&payload, state_db).await?;
        //     Ok(HotStuffTreeNode::genesis(payload, state_root))
        // } else {
        let payload = payload_provider.create_payload().await?;

        let state_root = payload_processor.process_payload(&payload, state_db).await?;
        Ok(HotStuffTreeNode::from_parent(
            parent,
            payload,
            state_root,
            view_id.as_u64() as u32,
        ))
        // }
    }

    async fn broadcast_proposal(
        &self,
        outbound: &mut TSpecification::OutboundService,
        committee: &[TSpecification::Addr],
        proposal: HotStuffTreeNode<TSpecification::Payload>,
        high_qc: QuorumCertificate,
        view_number: ViewId,
    ) -> Result<(), DigitalAssetError> {
        let message = HotStuffMessage::prepare(proposal, Some(high_qc), view_number, self.shard);
        outbound.broadcast(self.node_id.clone(), committee, message).await
    }

    fn does_extend(&self, node: &HotStuffTreeNode<TSpecification::Payload>, from: &TreeNodeHash) -> bool {
        from == node.parent()
    }

    fn is_safe_node<TUnitOfWork: ChainDbUnitOfWork>(
        &self,
        node: &HotStuffTreeNode<TSpecification::Payload>,
        quorum_certificate: &QuorumCertificate,
        chain_tx: &mut TUnitOfWork,
    ) -> Result<bool, StorageError> {
        let locked_qc = chain_tx.get_locked_qc()?;
        todo!("Fix safe node")
        // Ok(self.does_extend(node, locked_qc.node_hash()) || quorum_certificate.view_number() >
        // locked_qc.view_number())
    }

    async fn send_vote_to_leader(
        &self,
        nodes: HashMap<Shard, TreeNodeHash>,
        outbound: &mut TSpecification::OutboundService,
        view_leader: &TSpecification::Addr,
        view_number: ViewId,
        signing_service: &TSpecification::SigningService,
    ) -> Result<(), DigitalAssetError> {
        let mut message = HotStuffMessage::vote_prepare(nodes, view_number, self.shard);
        message.add_partial_sig(signing_service.sign(&self.node_id, &message.create_signature_challenge())?);
        outbound.send(self.node_id.clone(), view_leader.clone(), message).await
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use tari_common_types::types::FixedHash;

    use crate::{
        models::{AssetDefinition, Committee, HotStuffMessage, QuorumCertificate, TreeNodeHash, View, ViewId},
        services::{
            infrastructure_services::OutboundService,
            mocks::{
                create_public_key,
                mock_payload_processor,
                mock_signing_service,
                mock_static_payload_provider,
                MockChainStorageService,
                MockServiceSpecification,
            },
        },
        storage::{mocks::MockDbFactory, DbFactory},
        workers::states::{ConsensusWorkerStateEvent, Prepare},
    };

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn basic_test_as_leader() {
        // let mut inbound = mock_inbound();
        // let mut sender = inbound.create_sender();
        // let locked_qc = QuorumCertificate::genesis(TreeNodeHash::zero());
        // let contract_id = FixedHash::default();
        // let address_a = create_public_key();
        // let address_b = create_public_key();
        // let address_c = create_public_key();
        // let address_d = create_public_key();
        //
        // let mut state = Prepare::<MockServiceSpecification>::new(address_b.clone(), contract_id);
        // let current_view = View {
        //     view_id: ViewId(1),
        //     is_leader: true,
        // };
        // let timeout = Duration::from_secs(10);
        // let asset_definition = AssetDefinition::default();
        // let committee = Committee::new(vec![
        //     address_a.clone(),
        //     address_b.clone(),
        //     address_c.clone(),
        //     address_d.clone(),
        // ]);
        // let mut outbound = mock_outbound(committee.members.clone());
        // let mut outbound2 = outbound.clone();
        // let inbound = outbound.take_inbound(&address_b).unwrap();
        // let mut payload_provider = mock_static_payload_provider();
        // let signing_service = mock_signing_service();
        // let mut payload_processor = mock_payload_processor();
        // let chain_storage_service = MockChainStorageService::default();
        // let db_factory = MockDbFactory::default();
        // let chain_db = db_factory.get_or_create_chain_db(&contract_id.clone()).unwrap();
        // let chain_tx = chain_db.new_unit_of_work();
        // let mut state_tx = db_factory
        //     .get_or_create_state_db(&contract_id)
        //     .unwrap()
        //     .new_unit_of_work(current_view.view_id.as_u64());

        todo!()
        // let task = state.next_event(
        //     &current_view,
        //     timeout,
        //     &asset_definition,
        //     &committee,
        //     &inbound,
        //     &mut outbound,
        //     &mut payload_provider,
        //     &signing_service,
        //     &mut payload_processor,
        //     &chain_storage_service,
        //     chain_tx,
        //     &mut state_tx,
        //     &db_factory,
        // );
        //
        // outbound2
        //     .send(
        //         address_a.clone(),
        //         address_b.clone(),
        //         HotStuffMessage::new_view(locked_qc.clone(), ViewId(0), contract_id),
        //     )
        //     .await
        //     .unwrap();
        //
        // outbound2
        //     .send(
        //         address_c.clone(),
        //         address_b.clone(),
        //         HotStuffMessage::new_view(locked_qc.clone(), ViewId(0), contract_id),
        //     )
        //     .await
        //     .unwrap();
        //
        // outbound2
        //     .send(
        //         address_d.clone(),
        //         address_b.clone(),
        //         HotStuffMessage::new_view(locked_qc.clone(), ViewId(0), contract_id),
        //     )
        //     .await
        //     .unwrap();
        //
        // let event = task.await.unwrap();
        // assert_eq!(event, ConsensusWorkerStateEvent::Prepared);
    }
}
