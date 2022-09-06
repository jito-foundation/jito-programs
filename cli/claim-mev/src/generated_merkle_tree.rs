use std::{
    fmt::{Debug, Display, Formatter},
};

use log::*;
use serde::{Deserialize, Serialize};
use solana_client::{client_error::ClientError};
use solana_merkle_tree::MerkleTree;
use solana_sdk::{
    clock::Slot,
    pubkey::Pubkey,
    stake_history::Epoch,
};
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    AnchorError(#[from] anchor_lang::error::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    RpcError(#[from] ClientError),

}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GeneratedMerkleTreeCollection {
    pub generated_merkle_trees: Vec<GeneratedMerkleTree>,
    pub bank_hash: String,
    pub epoch: Epoch,
    pub slot: Slot,
}

#[derive(Eq, Debug, Hash, PartialEq, Deserialize, Serialize)]
pub struct GeneratedMerkleTree {
    pub tip_distribution_account: Pubkey,
    #[serde(skip_serializing, skip_deserializing)]
    pub merkle_tree: MerkleTree,
    pub tree_nodes: Vec<TreeNode>,
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
}

#[derive(Clone, Eq, Debug, Hash, PartialEq, Deserialize, Serialize)]
pub struct TreeNode {
    /// The account entitled to redeem.
    pub claimant: Pubkey,

    /// The amount this account is entitled to.
    pub amount: u64,

    /// The proof associated with this TreeNode
    pub proof: Option<Vec<[u8; 32]>>,
}
