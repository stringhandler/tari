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

use digest::Digest;
use tari_common_types::types::FixedHash;
use tari_core::transactions::transaction_components::SignerSignature;
use tari_crypto::common::Blake256;
use tari_dan_common_types::Shard;

use crate::models::{
    HotStuffMessageType,
    HotStuffTreeNode,
    Payload,
    QuorumCertificate,
    TreeNodeHash,
    ValidatorSignature,
    ViewId,
};

#[derive(Debug, Clone)]
pub struct HotStuffMessage<TPayload: Payload> {
    view_number: ViewId,
    shard: Shard,
    message_type: HotStuffMessageType,
    justify: Option<QuorumCertificate>,
    node: Option<HotStuffTreeNode<TPayload>>,
    node_hashes: Option<HashMap<Shard, TreeNodeHash>>,
    partial_sig: Option<ValidatorSignature>,
    checkpoint_signature: Option<SignerSignature>,
}

impl<TPayload: Payload> HotStuffMessage<TPayload> {
    pub fn new(
        view_number: ViewId,
        shard: Shard,
        message_type: HotStuffMessageType,
        justify: Option<QuorumCertificate>,
        node: Option<HotStuffTreeNode<TPayload>>,
        node_hashes: Option<HashMap<Shard, TreeNodeHash>>,
        partial_sig: Option<ValidatorSignature>,
        checkpoint_signature: Option<SignerSignature>,
    ) -> Self {
        Self {
            view_number,
            shard,
            message_type,
            justify,
            node,
            node_hashes,
            partial_sig,
            checkpoint_signature,
        }
    }

    pub fn new_view(prepare_qc: QuorumCertificate, view_number: ViewId, shard: Shard) -> Self {
        Self {
            shard,
            message_type: HotStuffMessageType::NewView,
            view_number,
            justify: Some(prepare_qc),
            node: None,
            partial_sig: None,
            checkpoint_signature: None,
            node_hashes: None,
        }
    }

    pub fn prepare(
        proposal: HotStuffTreeNode<TPayload>,
        high_qc: Option<QuorumCertificate>,
        view_number: ViewId,
        shard: Shard,
    ) -> Self {
        Self {
            message_type: HotStuffMessageType::Prepare,
            node: Some(proposal),
            justify: high_qc,
            view_number,
            shard,
            partial_sig: None,
            checkpoint_signature: None,
            node_hashes: None,
        }
    }

    pub fn vote_prepare(node_hashes: HashMap<Shard, TreeNodeHash>, view_number: ViewId, shard: Shard) -> Self {
        Self {
            message_type: HotStuffMessageType::Prepare,
            node_hashes: Some(node_hashes),
            view_number,
            shard,
            node: None,
            partial_sig: None,
            checkpoint_signature: None,
            justify: None,
        }
    }

    pub fn pre_commit(
        node: Option<HotStuffTreeNode<TPayload>>,
        prepare_qc: Option<QuorumCertificate>,
        view_number: ViewId,
        shard: Shard,
    ) -> Self {
        Self {
            message_type: HotStuffMessageType::PreCommit,
            node,
            justify: prepare_qc,
            view_number,
            shard,
            node_hashes: None,
            checkpoint_signature: None,
            partial_sig: None,
        }
    }

    pub fn vote_pre_commit(node_hashes: HashMap<Shard, TreeNodeHash>, view_number: ViewId, shard: Shard) -> Self {
        Self {
            message_type: HotStuffMessageType::PreCommit,
            node_hashes: Some(node_hashes),
            view_number,
            shard,
            node: None,
            partial_sig: None,
            checkpoint_signature: None,
            justify: None,
        }
    }

    pub fn commit(
        node: Option<HotStuffTreeNode<TPayload>>,
        pre_commit_qc: Option<QuorumCertificate>,
        view_number: ViewId,
        shard: Shard,
    ) -> Self {
        Self {
            message_type: HotStuffMessageType::Commit,
            node,
            justify: pre_commit_qc,
            view_number,
            shard,
            partial_sig: None,
            checkpoint_signature: None,
            node_hashes: None,
        }
    }

    pub fn vote_commit(
        node_hashes: HashMap<Shard, TreeNodeHash>,
        view_number: ViewId,
        shard: Shard,
        checkpoint_signature: SignerSignature,
    ) -> Self {
        Self {
            message_type: HotStuffMessageType::Commit,
            node_hashes: Some(node_hashes),
            view_number,
            shard,
            node: None,
            partial_sig: None,
            checkpoint_signature: Some(checkpoint_signature),
            justify: None,
        }
    }

    pub fn decide(
        node: Option<HotStuffTreeNode<TPayload>>,
        commit_qc: Option<QuorumCertificate>,
        view_number: ViewId,
        shard: Shard,
    ) -> Self {
        Self {
            message_type: HotStuffMessageType::Decide,
            node,
            justify: commit_qc,
            view_number,
            shard,
            partial_sig: None,
            checkpoint_signature: None,
            node_hashes: None,
        }
    }

    pub fn create_signature_challenge(&self) -> Vec<u8> {
        let mut b = Blake256::new()
            .chain(&[self.message_type.as_u8()])
            .chain(self.view_number.as_u64().to_le_bytes());
        if let Some(ref node) = self.node {
            b = b.chain(node.calculate_hash().as_bytes());
        } else if let Some(ref node_hash) = self.node_hashes {
            b = b.chain((node_hash.len() as u64).to_le_bytes());
            for (shard, hash) in node_hash {
                b = b.chain(shard.id.to_le_bytes());
                b = b.chain(hash.as_bytes());
            }
            // b = b.chain(node_hash.as_bytes());
        } else {
        }
        b.finalize().to_vec()
    }

    pub fn view_number(&self) -> ViewId {
        self.view_number
    }

    pub fn shard(&self) -> Shard {
        self.shard
    }

    pub fn node(&self) -> Option<&HotStuffTreeNode<TPayload>> {
        self.node.as_ref()
    }

    pub fn message_type(&self) -> HotStuffMessageType {
        self.message_type
    }

    pub fn justify(&self) -> Option<&QuorumCertificate> {
        self.justify.as_ref()
    }

    pub fn matches(&self, message_type: HotStuffMessageType, view_id: ViewId) -> bool {
        // from hotstuf spec
        self.message_type() == message_type && view_id == self.view_number()
    }

    pub fn add_partial_sig(&mut self, signature: ValidatorSignature) {
        self.partial_sig = Some(signature)
    }

    pub fn partial_sig(&self) -> Option<&ValidatorSignature> {
        self.partial_sig.as_ref()
    }

    pub fn checkpoint_signature(&self) -> Option<&SignerSignature> {
        self.checkpoint_signature.as_ref()
    }
}
