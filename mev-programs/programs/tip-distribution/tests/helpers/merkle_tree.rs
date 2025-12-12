// tests/utils/merkle_tree.rs

use solana_program::hash::hashv;

/// Test-only Merkle Tree implementation
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// Original leaf data (pre-hash)
    pub leafs: Vec<Vec<u8>>,

    /// Tree layers (already hashed)
    pub layers: Vec<Vec<[u8; 32]>>,
}

impl MerkleTree {
    /// Equivalent of `new MerkleTree(leafs)`
    pub fn new(leafs: Vec<Vec<u8>>) -> Self {
        let mut layers: Vec<Vec<[u8; 32]>> = Vec::new();

        // First layer = node hashes
        let mut hashes: Vec<[u8; 32]> = leafs.iter().map(|leaf| Self::node_hash(leaf)).collect();

        while !hashes.is_empty() {
            layers.push(hashes.clone());

            if hashes.len() == 1 {
                break;
            }

            let mut next_layer: Vec<[u8; 32]> = Vec::new();

            for i in (0..hashes.len()).step_by(2) {
                let first = hashes[i];
                let second = hashes.get(i + 1).copied();
                let combined = Self::internal_hash(first, second);
                next_layer.push(combined);
            }

            hashes = next_layer;
        }

        Self { leafs, layers }
    }

    /// sha256 helper (mirrors crypto.createHash("sha256"))
    #[inline]
    pub fn sha256(parts: &[&[u8]]) -> [u8; 32] {
        hashv(parts).to_bytes()
    }

    /// Equivalent of:
    /// sha256(0x00 || sha256(data))
    pub fn node_hash(data: &[u8]) -> [u8; 32] {
        let inner = Self::sha256(&[data]);
        Self::sha256(&[&[0x00], &inner])
    }

    /// Equivalent of:
    /// sha256(0x01 || min(first, second) || max(first, second))
    pub fn internal_hash(first: [u8; 32], second: Option<[u8; 32]>) -> [u8; 32] {
        let Some(second) = second else {
            return first;
        };

        let (fst, snd) = if first <= second {
            (first, second)
        } else {
            (second, first)
        };

        Self::sha256(&[&[0x01], &fst, &snd])
    }

    /// Equivalent of `getRoot()`
    pub fn get_root(&self) -> [u8; 32] {
        self.layers.last().expect("Merkle tree has no layers")[0]
    }

    /// Equivalent of `getProof(idx)`
    pub fn get_proof(&self, mut idx: usize) -> Vec<[u8; 32]> {
        let mut proof = Vec::new();

        for layer in &self.layers {
            let sibling = idx ^ 1;

            if sibling < layer.len() {
                proof.push(layer[sibling]);
            }

            idx /= 2;
        }

        proof
    }

    /// Equivalent of `verifyProof(idx, proof, root)`
    pub fn verify_proof(&self, idx: usize, proof: &[[u8; 32]], root: [u8; 32]) -> bool {
        let mut pair = Self::node_hash(&self.leafs[idx]);

        for p in proof {
            pair = Self::internal_hash(pair, Some(*p));
        }

        pair == root
    }

    /// Equivalent of `static verifyClaim(leaf, proof, root)`
    pub fn verify_claim(leaf: &[u8], proof: &[[u8; 32]], root: [u8; 32]) -> bool {
        let mut pair = Self::node_hash(leaf);

        for p in proof {
            pair = Self::internal_hash(pair, Some(*p));
        }

        pair == root
    }
}
