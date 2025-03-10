//! This module is where all PDA structs lives.

use std::mem::size_of;

use anchor_lang::prelude::*;

use crate::ErrorCode::{AccountValidationFailure, ArithmeticError};

#[account]
#[derive(Default)]
pub struct Config {
    /// Account with authority over this PDA.
    pub authority: Pubkey,

    /// We want to expire funds after some time so that validators can be refunded the rent.
    /// Expired funds will get transferred to this account.
    pub expired_funds_account: Pubkey,

    /// Specifies the number of epochs a merkle root is valid for before expiring.
    pub num_epochs_valid: u64,

    /// The maximum commission a validator can set on their distribution account.
    pub max_validator_commission_bps: u16,

    /// The bump used to generate this account
    pub bump: u8,
}

/// The account that validators register as **tip_receiver** with the tip-payment program.
#[account]
#[derive(Default)]
pub struct TipDistributionAccount {
    /// The validator's vote account, also the recipient of remaining lamports after
    /// upon closing this account.
    pub validator_vote_account: Pubkey,

    /// The only account authorized to upload a merkle-root for this account.
    pub merkle_root_upload_authority: Pubkey,

    /// The merkle root used to verify user claims from this account.
    pub merkle_root: Option<MerkleRoot>,

    /// Epoch for which this account was created.  
    pub epoch_created_at: u64,

    /// The commission basis points this validator charges.
    pub validator_commission_bps: u16,

    /// The epoch (upto and including) that tip funds can be claimed.
    pub expires_at: u64,

    /// The bump used to generate this account
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct MerkleRoot {
    /// The 256-bit merkle root.
    pub root: [u8; 32],

    /// Maximum number of funds that can ever be claimed from this [MerkleRoot].
    pub max_total_claim: u64,

    /// Maximum number of nodes that can ever be claimed from this [MerkleRoot].
    pub max_num_nodes: u64,

    /// Total funds that have been claimed.
    pub total_funds_claimed: u64,

    /// Number of nodes that have been claimed.
    pub num_nodes_claimed: u64,
}

const HEADER_SIZE: usize = 8;

impl Config {
    pub const SEED: &'static [u8] = b"CONFIG_ACCOUNT";
    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();

    pub fn validate(&self) -> Result<()> {
        const MAX_NUM_EPOCHS_VALID: u64 = 10;
        const MAX_VALIDATOR_COMMISSION_BPS: u16 = 10000;

        if self.num_epochs_valid == 0 || self.num_epochs_valid > MAX_NUM_EPOCHS_VALID {
            return Err(AccountValidationFailure.into());
        }

        if self.max_validator_commission_bps > MAX_VALIDATOR_COMMISSION_BPS {
            return Err(AccountValidationFailure.into());
        }

        let default_pubkey = Pubkey::default();
        if self.expired_funds_account == default_pubkey || self.authority == default_pubkey {
            return Err(AccountValidationFailure.into());
        }

        Ok(())
    }
}

impl TipDistributionAccount {
    pub const SEED: &'static [u8] = b"TIP_DISTRIBUTION_ACCOUNT";

    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();

    pub fn validate(&self) -> Result<()> {
        let default_pubkey = Pubkey::default();
        if self.validator_vote_account == default_pubkey
            || self.merkle_root_upload_authority == default_pubkey
        {
            return Err(AccountValidationFailure.into());
        }

        Ok(())
    }

    pub fn claim_expired(from: AccountInfo, to: AccountInfo) -> Result<u64> {
        let rent = Rent::get()?;
        let min_rent_lamports = rent.minimum_balance(from.data_len());

        let amount = from
            .lamports()
            .checked_sub(min_rent_lamports)
            .ok_or(ArithmeticError)?;
        Self::transfer_lamports(from, to, amount)?;

        Ok(amount)
    }

    pub fn claim(from: AccountInfo, to: AccountInfo, amount: u64) -> Result<()> {
        Self::transfer_lamports(from, to, amount)
    }

    fn transfer_lamports(from: AccountInfo, to: AccountInfo, amount: u64) -> Result<()> {
        // debit lamports
        **from.try_borrow_mut_lamports()? =
            from.lamports().checked_sub(amount).ok_or(ArithmeticError)?;
        // credit lamports
        **to.try_borrow_mut_lamports()? =
            to.lamports().checked_add(amount).ok_or(ArithmeticError)?;

        Ok(())
    }
}

// Epoch 751 had 1,286,573 delegations in the stake meta
//
// With current layout and fields:
//      data length: 88 bytes + 8 byte anchor header = 96 bytes
//      rent exempt: 0.00155904 SOL
//      1,286,573 x 0.00155904 = 2005.81876992 SOL per epoch
//
// if we make expires a u32, cut claim status payer and force single payer, and pack bytes so is_claimed takes a single byte:
//      data length: 45 bytes + 8 byte anchor header = 53 bytes
//      rent exempt: 0.00125976 SOL
//      1,286,573 x 0.00125976 = 1620.77320248 SOL per epoch
//
// 
/// Gives us an audit trail of who and what was claimed; also enforces and only-once claim by any party.
#[account]
#[derive(Default)]
pub struct ClaimStatus {
    /// If true, the tokens have been claimed.
    pub is_claimed: bool,

    /// Authority that claimed the tokens. Allows for delegated rewards claiming.
    pub claimant: Pubkey,

    /// The payer who created the claim.
    // REVIEW SAVE SPACE: Used when claiming rent. Can we make an assumption that rent is 
    //  always paid by Jito and can be returned to a single address? (Given the 
    //  `merkle_root_upload_authority` is a co-signer)
    pub claim_status_payer: Pubkey,

    /// Amount of funds claimed.
    pub amount: u64,

    /// The epoch (upto and including) that tip funds can be claimed.
    /// Copied since TDA can be closed, need to track to avoid making multiple claims
    // REVIEW SAVE SPACE: could store as a u32 (~2 day epochs = ~23,534,000 years).
    pub expires_at: u64,
}

impl ClaimStatus {
    pub const SEED: &'static [u8] = b"CLAIM_STATUS";

    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();
}

/// Singleton account that allows overriding TDA's merkle upload authority
#[account]
#[derive(Default)]
pub struct MerkleRootUploadConfig {
    /// The authority that overrides the TipDistributionAccount merkle_root_upload_authority
    pub override_authority: Pubkey,

    /// The original merkle root upload authority that can be changed to the new overrided 
    /// authority. E.g. Jito Labs authority GZctHpWXmsZC1YHACTGGcHhYxjdRqQvTpYkb9LMvxDib
    pub original_upload_authority: Pubkey,

    /// The bump used to generate this account
    pub bump: u8,
}

impl MerkleRootUploadConfig {
    pub const SEED: &'static [u8] = b"ROOT_UPLOAD_CONFIG";

    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();
}
