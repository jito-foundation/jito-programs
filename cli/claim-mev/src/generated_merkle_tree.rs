#[allow(unused_imports)]
use std::{
    fmt::{Debug, Display, Formatter},
    ops::{Div, Mul},
    sync::Arc,
};

use anchor_lang::AccountDeserialize;
use log::*;
use serde::{Deserialize, Serialize};
use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_merkle_tree::MerkleTree;
use solana_runtime::bank::Bank;
use solana_sdk::{
    account::{AccountSharedData, ReadableAccount},
    clock::Slot,
    pubkey::Pubkey,
    stake_history::Epoch,
};
use thiserror::Error as ThisError;
use tip_distribution::state::TipDistributionAccount;

#[allow(dead_code)]
#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    AnchorError(#[from] anchor_lang::error::Error),

    Base58DecodeError,

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    RpcError(#[from] ClientError),
    SnapshotSlotNotFound,
    CheckedMathError,
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

/// Convenience wrapper around [TipDistributionAccount]
pub struct TipDistributionAccountWrapper {
    pub tip_distribution_account: TipDistributionAccount,
    pub account_data: AccountSharedData,
    pub tip_distribution_account_pubkey: Pubkey,
}

// TODO: move to program's sdk
#[allow(dead_code)]
pub fn derive_tip_distribution_account_address(
    tip_distribution_program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    epoch: Epoch,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            TipDistributionAccount::SEED,
            vote_pubkey.to_bytes().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        tip_distribution_program_id,
    )
}

pub trait AccountFetcher {
    fn fetch_account(&self, pubkey: &Pubkey) -> Result<Option<AccountSharedData>, Error>;
}

/// Fetches and deserializes the vote_pubkey's corresponding [TipDistributionAccount].
#[allow(dead_code)]
pub fn fetch_and_deserialize_tip_distribution_account(
    account_fetcher: Arc<Box<dyn AccountFetcher>>,
    vote_pubkey: &Pubkey,
    tip_distribution_program_id: &Pubkey,
    epoch: Epoch,
) -> Result<Option<TipDistributionAccountWrapper>, Error> {
    let tip_distribution_account_pubkey =
        derive_tip_distribution_account_address(tip_distribution_program_id, vote_pubkey, epoch).0;

    match account_fetcher.fetch_account(&tip_distribution_account_pubkey)? {
        None => {
            warn!(
                "TipDistributionAccount not found for vote_pubkey {}, epoch {}, tip_distribution_account_pubkey {}",
                vote_pubkey,
                epoch,
                tip_distribution_account_pubkey,
            );
            Ok(None)
        }
        Some(account_data) => Ok(Some(TipDistributionAccountWrapper {
            tip_distribution_account: TipDistributionAccount::try_deserialize(
                &mut account_data.data(),
            )?,
            account_data,
            tip_distribution_account_pubkey,
        })),
    }
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

/// Calculate validator fee denominated in lamports
#[allow(dead_code)]
pub fn calc_validator_fee(total_tips: u64, validator_commission_bps: u16) -> u64 {
    let validator_commission_rate =
        math::fee_tenth_of_bps(((validator_commission_bps as u64).checked_mul(10).unwrap()) as u64);
    let validator_fee: math::U64F64 = validator_commission_rate.mul_u64(total_tips);

    validator_fee
        .floor()
        .checked_add((validator_fee.frac_part() != 0) as u64)
        .unwrap()
}

mod math {
    /// copy-pasta from [here](https://github.com/project-serum/serum-dex/blob/e00bb9e6dac0a1fff295acb034722be9afc1eba3/dex/src/fees.rs#L43)
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub(crate) struct U64F64(u128);

    #[allow(dead_code)]
    impl U64F64 {
        const ONE: Self = U64F64(1 << 64);

        pub(crate) fn add(self, other: U64F64) -> U64F64 {
            U64F64(self.0.checked_add(other.0).unwrap())
        }

        pub(crate) fn div(self, other: U64F64) -> u128 {
            self.0.checked_div(other.0).unwrap()
        }

        pub(crate) fn mul_u64(self, other: u64) -> U64F64 {
            U64F64(self.0.checked_mul(other as u128).unwrap())
        }

        /// right shift 64
        pub(crate) fn floor(self) -> u64 {
            (self.0.checked_div(2u128.checked_pow(64).unwrap()).unwrap()) as u64
        }

        pub(crate) fn frac_part(self) -> u64 {
            self.0 as u64
        }

        /// left shift 64
        pub(crate) fn from_int(n: u64) -> Self {
            U64F64(
                (n as u128)
                    .checked_mul(2u128.checked_pow(64).unwrap())
                    .unwrap(),
            )
        }
    }

    pub(crate) fn fee_tenth_of_bps(tenth_of_bps: u64) -> U64F64 {
        U64F64(
            ((tenth_of_bps as u128)
                .checked_mul(2u128.checked_pow(64).unwrap())
                .unwrap())
            .checked_div(100_000)
            .unwrap(),
        )
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::bs58;
    use tip_distribution::merkle_proof;

    use super::*;

    #[test]
    fn test_merkle_tree_verify() {
        // Create the merkle tree and proofs
        let acct_0 = bs58::encode(Pubkey::new_unique().as_ref()).into_string();
        let acct_1 = bs58::encode(Pubkey::new_unique().as_ref()).into_string();

        let tree_nodes = vec![
            TreeNode {
                claimant: acct_0.parse().unwrap(),
                amount: 151_507,
                proof: None,
            },
            TreeNode {
                claimant: acct_1.parse().unwrap(),
                amount: 176_624,
                proof: None,
            },
        ];

        // First the nodes are hashed and merkle tree constructed
        let hashed_nodes: Vec<[u8; 32]> = tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();
        let mk = MerkleTree::new(&hashed_nodes[..], true);
        let root = mk.get_root().expect("to have valid root").to_bytes();

        // verify first node
        let node = solana_program::hash::hashv(&[&[0u8], &hashed_nodes[0]]);
        let proof = get_proof(&mk, 0);
        assert!(merkle_proof::verify(proof, root, node.to_bytes()));

        // verify second node
        let node = solana_program::hash::hashv(&[&[0u8], &hashed_nodes[1]]);
        let proof = get_proof(&mk, 1);
        assert!(merkle_proof::verify(proof, root, node.to_bytes()));
    }

    #[test]
    fn test_new_from_stake_meta_collection_happy_path() {
        let b58_merkle_root_upload_authority =
            bs58::encode(Pubkey::new_unique().as_ref()).into_string();

        let (tda_0, tda_1) = (
            bs58::encode(Pubkey::new_unique().as_ref()).into_string(),
            bs58::encode(Pubkey::new_unique().as_ref()).into_string(),
        );

        let stake_account_0 = bs58::encode(Pubkey::new_unique().as_ref()).into_string();
        let stake_account_1 = bs58::encode(Pubkey::new_unique().as_ref()).into_string();
        let stake_account_2 = bs58::encode(Pubkey::new_unique().as_ref()).into_string();
        let stake_account_3 = bs58::encode(Pubkey::new_unique().as_ref()).into_string();

        let validator_vote_account_0 = bs58::encode(Pubkey::new_unique().as_ref()).into_string();
        let validator_vote_account_1 = bs58::encode(Pubkey::new_unique().as_ref()).into_string();

        println!("test stake_account {}", stake_account_0);
        println!("test stake_account {}", stake_account_1);
        println!("test stake_account {}", stake_account_2);
        println!("test stake_account {}", stake_account_3);

        let stake_meta_collection = StakeMetaCollection {
            stake_metas: vec![
                StakeMeta {
                    validator_vote_account: validator_vote_account_0.clone(),
                    maybe_tip_distribution_meta: Some(TipDistributionMeta {
                        merkle_root_upload_authority: b58_merkle_root_upload_authority.clone(),
                        tip_distribution_account: tda_0.clone(),
                        total_tips: 1_900_122_111_000,
                        validator_fee_bps: 100,
                    }),
                    delegations: vec![
                        Delegation {
                            stake_account: stake_account_0.clone(),
                            amount_delegated: 123_999_123_555,
                        },
                        Delegation {
                            stake_account: stake_account_1.clone(),
                            amount_delegated: 144_555_444_556,
                        },
                    ],
                    total_delegated: 1_555_123_000_333_454_000,
                    commission: 100,
                },
                StakeMeta {
                    validator_vote_account: validator_vote_account_1.clone(),
                    maybe_tip_distribution_meta: Some(TipDistributionMeta {
                        merkle_root_upload_authority: b58_merkle_root_upload_authority.clone(),
                        tip_distribution_account: tda_1.clone(),
                        total_tips: 1_900_122_111_333,
                        validator_fee_bps: 200,
                    }),
                    delegations: vec![
                        Delegation {
                            stake_account: stake_account_2.clone(),
                            amount_delegated: 224_555_444,
                        },
                        Delegation {
                            stake_account: stake_account_3.clone(),
                            amount_delegated: 700_888_944_555,
                        },
                    ],
                    total_delegated: 2_565_318_909_444_123,
                    commission: 10,
                },
            ],
            tip_distribution_program_id: bs58::encode(Pubkey::new_unique().as_ref()).into_string(),
            bank_hash: solana_sdk::hash::Hash::new_unique().to_string(),
            epoch: 100,
            slot: 2_000_000,
        };

        let merkle_tree_collection = GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
            stake_meta_collection.clone(),
            b58_merkle_root_upload_authority.parse().unwrap(),
        )
        .unwrap();

        assert_eq!(stake_meta_collection.epoch, merkle_tree_collection.epoch);
        assert_eq!(
            stake_meta_collection.bank_hash,
            merkle_tree_collection.bank_hash
        );
        assert_eq!(stake_meta_collection.slot, merkle_tree_collection.slot);
        assert_eq!(
            stake_meta_collection.stake_metas.len(),
            merkle_tree_collection.generated_merkle_trees.len()
        );

        let tree_nodes = vec![
            TreeNode {
                claimant: validator_vote_account_0.parse().unwrap(),
                amount: 19_001_221_110,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_0.parse().unwrap(),
                amount: 149_992,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_1.parse().unwrap(),
                amount: 174_858,
                proof: None,
            },
        ];
        let hashed_nodes: Vec<[u8; 32]> = tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();
        let gmt_0 = GeneratedMerkleTree {
            tip_distribution_account: tda_0.parse().unwrap(),
            merkle_tree: MerkleTree::new(&hashed_nodes[..], true),
            tree_nodes,
            max_total_claim: stake_meta_collection.stake_metas[0]
                .clone()
                .maybe_tip_distribution_meta
                .unwrap()
                .total_tips,
            max_num_nodes: 3,
        };

        let tree_nodes = vec![
            TreeNode {
                claimant: validator_vote_account_1.parse().unwrap(),
                amount: 38_002_442_227,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_2.parse().unwrap(),
                amount: 163_000,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_3.parse().unwrap(),
                amount: 508_762_900,
                proof: None,
            },
        ];
        let hashed_nodes: Vec<[u8; 32]> = tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();
        let gmt_1 = GeneratedMerkleTree {
            tip_distribution_account: tda_1.parse().unwrap(),
            merkle_tree: MerkleTree::new(&hashed_nodes[..], true),
            tree_nodes,
            max_total_claim: stake_meta_collection.stake_metas[1]
                .clone()
                .maybe_tip_distribution_meta
                .unwrap()
                .total_tips,
            max_num_nodes: 3,
        };

        let expected_generated_merkle_trees = vec![gmt_0, gmt_1];
        let actual_generated_merkle_trees = merkle_tree_collection.generated_merkle_trees;

        expected_generated_merkle_trees
            .iter()
            .for_each(|expected_gmt| {
                let actual_gmt = actual_generated_merkle_trees
                    .iter()
                    .find(|gmt| {
                        gmt.tip_distribution_account == expected_gmt.tip_distribution_account
                    })
                    .unwrap();

                assert_eq!(expected_gmt.max_num_nodes, actual_gmt.max_num_nodes);
                assert_eq!(expected_gmt.max_total_claim, actual_gmt.max_total_claim);
                assert_eq!(
                    expected_gmt.tip_distribution_account,
                    actual_gmt.tip_distribution_account
                );
                assert_eq!(expected_gmt.tree_nodes.len(), actual_gmt.tree_nodes.len());
                expected_gmt
                    .tree_nodes
                    .iter()
                    .for_each(|expected_tree_node| {
                        let actual_tree_node = actual_gmt
                            .tree_nodes
                            .iter()
                            .find(|tree_node| tree_node.claimant == expected_tree_node.claimant)
                            .unwrap();
                        assert_eq!(expected_tree_node.amount, actual_tree_node.amount);
                    });
                assert_eq!(
                    expected_gmt.merkle_tree.get_root().unwrap(),
                    actual_gmt.merkle_tree.get_root().unwrap()
                );
            });
    }
}
