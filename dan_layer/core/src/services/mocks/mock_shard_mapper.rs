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

use std::{collections::HashMap, ops::Range};

use tari_dan_common_types::{Shard, ShardKey};

use crate::services::infrastructure_services::NodeAddressable;

#[derive(Clone, Debug)]
pub struct MockShardMapper<TAddr: NodeAddressable> {
    shard_allocation: HashMap<Shard, Vec<TAddr>>,
    shard_map: Vec<(Range<u8>, Shard)>,
}

impl<TAddr: NodeAddressable> MockShardMapper<TAddr> {
    pub fn new() -> Self {
        Self {
            shard_allocation: HashMap::new(),
            shard_map: vec![],
        }
    }

    pub fn assign(&mut self, shard: Shard, range: Range<u8>, node: TAddr) {
        self.shard_allocation.entry(shard.clone()).or_insert(vec![]).push(node);
        self.shard_map.push((range, shard));
    }

    pub fn get_shards(&self) -> Vec<Vec<TAddr>> {
        let mut result = vec![];
        let mut keys: Vec<&Shard> = self.shard_allocation.keys().collect();
        for s in keys {
            result.push(self.shard_allocation.get(s).unwrap().clone());
        }

        result
    }

    pub fn get_nodes_for_shard(&self, shard: &Shard) -> Option<Vec<TAddr>> {
        self.shard_allocation.get(shard).map(|v| v.clone())
    }

    pub fn find_shard_for(&self, node: &TAddr) -> Option<Shard> {
        for (shard, nodes) in self.shard_allocation.iter() {
            if nodes.contains(node) {
                return Some(shard.clone());
            }
        }
        None
    }

    pub fn get_shard_for_key(&self, key: &ShardKey) -> Option<Shard> {
        for (range, shard) in self.shard_map.iter() {
            if range.contains(&key.0[0]) {
                return Some(shard.clone());
            }
        }
        None
    }
}
