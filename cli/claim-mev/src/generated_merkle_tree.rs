use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
};

use log::*;
use serde::{Deserialize, Serialize};
use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_merkle_tree::MerkleTree;
use solana_runtime::bank::Bank;
use solana_sdk::{
    account::{AccountSharedData},
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StakeMetaCollection {
    /// List of [StakeMeta].
    pub stake_metas: Vec<StakeMeta>,

    /// base58 encoded tip-distribution program id.
    pub tip_distribution_program_id: String,

    /// Base58 encoded bank hash this object was generated at.
    pub bank_hash: String,

    /// Epoch for which this object was generated for.
    pub epoch: Epoch,

    /// Slot at which this object was generated.
    pub slot: Slot,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct StakeMeta {
    /// The validator's base58 encoded vote account.
    pub validator_vote_account: String,

    /// The validator's tip-distribution meta if it exists.
    pub maybe_tip_distribution_meta: Option<TipDistributionMeta>,

    /// Delegations to this validator.
    pub delegations: Vec<Delegation>,

    /// The total amount of delegations to the validator.
    pub total_delegated: u64,

    /// The validator's delegation commission rate as a percentage between 0-100.
    pub commission: u8,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct TipDistributionMeta {
    /// The account authorized to generate and upload a merkle_root for the validator.
    pub merkle_root_upload_authority: String,

    /// The validator's base58 encoded [TipDistributionAccount].
    pub tip_distribution_account: String,

    /// The validator's total tips in the [TipDistributionAccount].
    pub total_tips: u64,

    /// The validator's cut of tips from [TipDistributionAccount], calculated from the on-chain
    /// commission fee bps.
    pub validator_fee_bps: u16,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct Delegation {
    /// The stake account of interest base58 encoded.
    pub stake_account: String,

    /// Amount delegated by this account.
    pub amount_delegated: u64,
}


pub trait AccountFetcher {
    fn fetch_account(&self, pubkey: &Pubkey) -> Result<Option<AccountSharedData>, Error>;
}


struct BankAccountFetcher {
    bank: Arc<Bank>,
}

impl AccountFetcher for BankAccountFetcher {
    /// Fetches the vote_pubkey's corresponding [TipDistributionAccount] from the accounts DB.
    fn fetch_account(&self, pubkey: &Pubkey) -> Result<Option<AccountSharedData>, Error> {
        Ok(self.bank.get_account(pubkey))
    }
}

struct RpcAccountFetcher {
    rpc_client: RpcClient,
}

impl AccountFetcher for RpcAccountFetcher {
    /// Fetches the vote_pubkey's corresponding [TipDistributionAccount] from an RPC node.
    fn fetch_account(&self, pubkey: &Pubkey) -> Result<Option<AccountSharedData>, Error> {
        match self
            .rpc_client
            .get_account_with_commitment(pubkey, self.rpc_client.commitment())
        {
            Ok(resp) => Ok(resp.value.map(|a| a.into())),
            Err(e) => {
                error!("error fetching account {}", e);
                Err(e.into())
            }
        }
    }
}
