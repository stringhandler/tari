// Copyright 2023. The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use digest::{consts::U32, Digest};

use crate::{sparse_merkle_tree::LeafNode, Hash};

use super::{DeleteResult, Node, NodeHash, NodeKey, SMTError, SparseMerkleTree, ValueHash};

pub struct SmtSnapshot<'a, H: Digest<OutputSize = U32>> {
    root: &'a SparseMerkleTree<H>,
    is_dirty: bool,
    inserted_nodes: HashMap<NodeKey, Node<H>>,
    deleted_nodes: HashMap<NodeKey, Node<H>>,
    // changes: Vec<SmtOperation>,
}

impl<'a, H: Digest<OutputSize = U32>> SmtSnapshot<'a, H> {
    pub fn new(root: &'a SparseMerkleTree<H>) -> Self {
        SmtSnapshot {
            root,
            is_dirty: false,
            inserted_nodes: HashMap::new(),
            deleted_nodes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: NodeKey, value: ValueHash) -> Result<(), SMTError> {
        if self.root.get(&key)?.is_some() {
            return Err(SMTError::KeyExists);
        }
        if self.inserted_nodes.contains_key(&key) {
            return Err(SMTError::KeyExists);
        }
        self.is_dirty = true;
        let new_leaf = LeafNode::<H>::new(key, value);
        // self.inserted_nodes.insert(key, new_leaf);
        todo!("add operation")
        // self.root.upsert(key.into(), value.into())
    }

    pub fn delete(&mut self, key: &NodeKey) -> Result<DeleteResult, SMTError> {
        todo!("add operation")
        // self.root.delete(&key.into())
    }

    pub fn hash(&self) -> NodeHash {
        if self.is_dirty {
            todo!()
        } else {
            self.root.unsafe_hash().clone()
        }
    }
}

#[cfg(test)]
mod test {
    use blake2::Blake2b;

    use super::*;
    fn short_key(v: u8) -> NodeKey {
        let mut key = [0u8; 32];
        key[0] = v;
        NodeKey::from(key)
    }

    #[test]
    fn test_root_hash() {
        let mut smt = super::SparseMerkleTree::<Blake2b<U32>>::new();
        let key = short_key(1);
        let value = ValueHash::from([1u8; 32]);
        smt.insert(key, value).unwrap();
        assert_eq!(
            smt.hash().to_string(),
            "f0ba9d3fa2b32a56d356b851098a2c7cb077f56735371f98eabccd5ffb9da689"
        );
        let snapshot = smt.create_snapshot();
        assert_eq!(
            snapshot.hash().to_string(),
            "f0ba9d3fa2b32a56d356b851098a2c7cb077f56735371f98eabccd5ffb9da689"
        );
    }
    #[test]
    fn test_snapshot_insert_node() {
        let mut smt = super::SparseMerkleTree::<Blake2b<U32>>::new();
        let key = short_key(1);
        let value = ValueHash::from([1u8; 32]);
        assert_eq!(
            smt.hash().to_string(),
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
        let mut snapshot = smt.create_snapshot();
        snapshot.insert(key, value).unwrap();
        assert_eq!(
            snapshot.hash().to_string(),
            "f0ba9d3fa2b32a56d356b851098a2c7cb077f56735371f98eabccd5ffb9da689"
        );
    }
}
