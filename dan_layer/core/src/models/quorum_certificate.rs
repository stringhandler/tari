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

use tari_dan_common_types::Shard;

use crate::{
    models::{HotStuffMessageType, TreeNodeHash, ValidatorSignature, ViewId},
    storage::chain::DbQc,
};

#[derive(Debug, Clone)]
pub struct QuorumCertificate {
    message_type: HotStuffMessageType,
    node_hashes: HashMap<Shard, TreeNodeHash>,
    view_number: ViewId,
    signatures: Vec<ValidatorSignature>,
}

impl QuorumCertificate {
    pub fn new(
        message_type: HotStuffMessageType,
        view_number: ViewId,
        node_hashes: HashMap<Shard, TreeNodeHash>,
        signatures: Vec<ValidatorSignature>,
    ) -> Self {
        Self {
            message_type,
            node_hashes,
            view_number,
            signatures,
        }
    }

    pub fn genesis(node_hash: TreeNodeHash) -> Self {
        Self {
            message_type: HotStuffMessageType::Genesis,
            node_hashes: HashMap::new(),
            view_number: 0.into(),
            signatures: vec![],
        }
    }

    pub fn node_hashes(&self) -> &HashMap<Shard, TreeNodeHash> {
        &self.node_hashes
    }

    pub fn node_hash(&self, shard: Shard) -> Option<&TreeNodeHash> {
        self.node_hashes.get(&shard)
    }

    pub fn view_number(&self) -> ViewId {
        self.view_number
    }

    pub fn message_type(&self) -> HotStuffMessageType {
        self.message_type
    }

    pub fn signatures(&self) -> &[ValidatorSignature] {
        self.signatures.as_slice()
    }

    pub fn add_sig(&mut self, sig: ValidatorSignature) {
        self.signatures.add(sig)
    }

    pub fn matches(&self, message_type: HotStuffMessageType, view_id: ViewId) -> bool {
        // from hotstuf spec
        self.message_type() == message_type && view_id == self.view_number()
    }
}

impl From<DbQc> for QuorumCertificate {
    fn from(rec: DbQc) -> Self {
        Self {
            message_type: rec.message_type,
            node_hashes: rec.node_hash,
            view_number: rec.view_number,
            signatures: rec.signatures,
        }
    }
}
