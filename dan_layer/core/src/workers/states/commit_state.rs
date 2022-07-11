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

use log::*;
use rand::rngs::OsRng;
use tari_common_types::types::{Commitment, FixedHash, PrivateKey};
use tari_core::transactions::transaction_components::SignerSignature;
use tari_crypto::keys::SecretKey;
use tari_dan_common_types::Shard;
use tokio::time::{sleep, Duration};

use crate::{
    digital_assets_error::DigitalAssetError,
    models::{
        CheckpointChallenge,
        Committee,
        HotStuffMessage,
        HotStuffMessageType,
        MergedVoteBuilder,
        QuorumCertificate,
        TreeNodeHash,
        View,
        ViewId,
    },
    services::{
        infrastructure_services::{InboundConnectionService, OutboundService},
        CommitteeManager,
        ServiceSpecification,
        SigningService,
    },
    storage::chain::ChainDbUnitOfWork,
    workers::states::ConsensusWorkerStateEvent,
};

const LOG_TARGET: &str = "tari::dan::workers::states::commit";

// TODO: This is very similar to pre-commit state
pub struct CommitState<TSpecification: ServiceSpecification> {
    node_id: TSpecification::Addr,
    contract_id: FixedHash,
    shard: Shard,
    committee: Committee<TSpecification::Addr>,
    received_new_view_messages: HashMap<TSpecification::Addr, HotStuffMessage<TSpecification::Payload>>,
    merged_vote_builder: MergedVoteBuilder,
}

impl<TSpecification: ServiceSpecification> CommitState<TSpecification> {
    pub fn new(
        node_id: TSpecification::Addr,
        contract_id: FixedHash,
        shard: Shard,
        committee: Committee<TSpecification::Addr>,
    ) -> Self {
        Self {
            node_id,
            contract_id,
            shard,
            committee,
            received_new_view_messages: HashMap::new(),
            merged_vote_builder: MergedVoteBuilder::new(),
        }
    }

    pub async fn next_event<TUnitOfWork: ChainDbUnitOfWork>(
        &mut self,
        timeout: Duration,
        current_view: &View,
        inbound_services: &TSpecification::InboundConnectionService,
        outbound_service: &mut TSpecification::OutboundService,
        signing_service: &TSpecification::SigningService,
        mut unit_of_work: TUnitOfWork,
        committee_manager: &TSpecification::CommitteeManager,
    ) -> Result<ConsensusWorkerStateEvent, DigitalAssetError> {
        self.received_new_view_messages.clear();
        let timeout = sleep(timeout);
        futures::pin_mut!(timeout);
        loop {
            tokio::select! {
               r  = inbound_services.wait_for_message(HotStuffMessageType::PreCommit, current_view.view_id()) => {
               let (from, message) = r?;
               if current_view.is_leader() {
                  if let Some(result) = self.process_leader_message(current_view, message.clone(), &from, outbound_service, committee_manager).await?{
                      break Ok(result);
                  }
              }
            },
            r =  inbound_services.wait_for_qc(HotStuffMessageType::PreCommit, current_view.view_id()) => {
                let (from, message) = r?;
                let leader = self.committee.leader_for_view(current_view.view_id).clone();
                if let Some(result) = self.process_replica_message(&message, current_view, &from, &leader,  outbound_service, signing_service, &mut unit_of_work).await? {
                    break Ok(result);
                }
            }
            _ = &mut timeout =>  {
                  break Ok(ConsensusWorkerStateEvent::TimedOut);
              }
            }
        }
    }

    async fn process_leader_message(
        &mut self,
        current_view: &View,
        message: HotStuffMessage<TSpecification::Payload>,
        sender: &TSpecification::Addr,
        outbound: &mut TSpecification::OutboundService,
        committee_manager: &TSpecification::CommitteeManager,
    ) -> Result<Option<ConsensusWorkerStateEvent>, DigitalAssetError> {
        if !message.matches(HotStuffMessageType::PreCommit, current_view.view_id) {
            return Ok(None);
        }

        // TODO: This might need to be checked in the QC rather
        if self.received_new_view_messages.contains_key(sender) {
            warn!(target: LOG_TARGET, "Already received message from {:?}", &sender);
            return Ok(None);
        }

        if !committee_manager.current_committee()?.contains(sender) {
            warn!(target: LOG_TARGET, "Received message from non-member: {:?}", sender);
            return Ok(None);
        }
        todo!(
            "Need to check the signatures of all the committees, otherwise just the leader of other shards can be \
             byzantine"
        );

        self.received_new_view_messages.insert(sender.clone(), message);

        if self.received_new_view_messages.len() >= self.committee.consensus_threshold() {
            debug!(
                target: LOG_TARGET,
                "[COMMIT] Consensus has been reached with {:?} out of {} votes",
                self.received_new_view_messages.len(),
                self.committee.len(),
            );

            if let Some(qc) = self.create_qc(current_view) {
                self.broadcast(outbound, qc, current_view.view_id).await?;
                return Ok(None); // Replica will move this on
            }
            warn!(target: LOG_TARGET, "committee did not agree on node");
            Ok(None)
        } else {
            debug!(
                target: LOG_TARGET,
                "[COMMIT] Consensus has NOT YET been reached with {:?} out of {} votes",
                self.received_new_view_messages.len(),
                self.committee.len()
            );
            Ok(None)
        }
    }

    async fn broadcast(
        &self,
        outbound: &mut TSpecification::OutboundService,
        pre_commit_qc: QuorumCertificate,
        view_number: ViewId,
    ) -> Result<(), DigitalAssetError> {
        let message = HotStuffMessage::commit(None, Some(pre_commit_qc), view_number, self.contract_id);
        outbound
            .broadcast(self.node_id.clone(), self.committee.members.as_slice(), message)
            .await
    }

    fn generate_checkpoint_signature(&self) -> SignerSignature {
        // TODO: wire in the signer secret (probably node identity)
        let signer_secret = PrivateKey::random(&mut OsRng);
        // TODO: Validators should have agreed on a checkpoint commitment and included this in the signature for base
        //       layer validation
        let commitment = Commitment::default();
        // TODO: We need the finalized state root to be able to produce a signature
        let state_root = FixedHash::zero();
        // TODO: Load next checkpoint number from db
        let checkpoint_number = 0;

        let challenge = CheckpointChallenge::new(&self.contract_id, &commitment, state_root, checkpoint_number);
        SignerSignature::sign(&signer_secret, challenge)
    }

    fn create_qc(&self, current_view: &View) -> Option<QuorumCertificate> {
        // TODO: This can be done in one loop instead of two
        let mut node_hash = None;
        for message in self.received_new_view_messages.values() {
            node_hash = match node_hash {
                None => message.node_hash().copied(),
                Some(n) => {
                    if let Some(m_node) = message.node_hash() {
                        if &n != m_node {
                            unimplemented!("Nodes did not match");
                        }
                        Some(*m_node)
                    } else {
                        Some(n)
                    }
                },
            };
        }

        let node_hash = node_hash.unwrap();
        let mut qc = QuorumCertificate::new(HotStuffMessageType::PreCommit, current_view.view_id, node_hash, None);
        for message in self.received_new_view_messages.values() {
            qc.combine_sig(message.partial_sig().unwrap())
        }
        Some(qc)
    }

    async fn process_replica_message<TUnitOfWork: ChainDbUnitOfWork>(
        &mut self,
        message: &HotStuffMessage<TSpecification::Payload>,
        current_view: &View,
        from: &TSpecification::Addr,
        view_leader: &TSpecification::Addr,
        outbound: &mut TSpecification::OutboundService,
        signing_service: &TSpecification::SigningService,
        unit_of_work: &mut TUnitOfWork,
    ) -> Result<Option<ConsensusWorkerStateEvent>, DigitalAssetError> {
        if let Some(justify) = message.justify() {
            if !justify.matches(HotStuffMessageType::PreCommit, current_view.view_id) {
                warn!(
                    target: LOG_TARGET,
                    "Wrong justify message type received, log: {} {:?} {}",
                    &self.node_id,
                    &justify.message_type(),
                    current_view.view_id
                );
                return Ok(None);
            }
            // if message.node().is_none() {
            //     unimplemented!("Empty message");
            // }

            if from != view_leader {
                warn!(target: LOG_TARGET, "Message not from leader");
                return Ok(None);
            }

            unit_of_work.set_locked_qc(justify)?;
            self.send_vote_to_leader(
                *justify.node_hash(),
                outbound,
                view_leader,
                current_view.view_id,
                signing_service,
            )
            .await?;
            Ok(Some(ConsensusWorkerStateEvent::Committed))
        } else {
            warn!(target: LOG_TARGET, "received non justify message");
            Ok(None)
        }
    }

    async fn send_vote_to_leader(
        &self,
        node: TreeNodeHash,
        outbound: &mut TSpecification::OutboundService,
        view_leader: &TSpecification::Addr,
        view_number: ViewId,
        signing_service: &TSpecification::SigningService,
    ) -> Result<(), DigitalAssetError> {
        let checkpoint_signature = self.generate_checkpoint_signature();
        let mut message = HotStuffMessage::vote_commit(node, view_number, self.contract_id, checkpoint_signature);
        message.add_partial_sig(signing_service.sign(&self.node_id, &message.create_signature_challenge())?);
        outbound.send(self.node_id.clone(), view_leader.clone(), message).await
    }
}
