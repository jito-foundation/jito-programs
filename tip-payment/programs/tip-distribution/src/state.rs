//! This module is where all PDA structs lives.

use std::mem::size_of;

use anchor_lang::prelude::*;

use crate::ErrorCode::{AccountValidationFailure, RentExemptViolation};

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

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
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
        let default_pubkey = Pubkey::default();
        // validators cannot set commission to be greater than 100%
        if self.max_validator_commission_bps > 10000 ||
            // prevent from accidentally setting these to the System program
            || self.expired_funds_account == default_pubkey
            || self.authority == default_pubkey
        {
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
        let min_rent_lamports = rent.minimum_balance(TipDistributionAccount::SIZE);
        let amount = from.lamports().checked_sub(min_rent_lamports).unwrap();
        Self::checked_transfer(from, to, amount, min_rent_lamports)?;

        Ok(amount)
    }

    pub fn claim(from: AccountInfo, to: AccountInfo, amount: u64) -> Result<()> {
        let rent = Rent::get()?;
        let min_rent_lamports = rent.minimum_balance(TipDistributionAccount::SIZE);

        Self::checked_transfer(from, to, amount, min_rent_lamports)
    }

    /// Transfers funds from-from and to-to. Returns an error if from ends up with less than what's
    /// required for rent exemption.
    fn checked_transfer(
        from: AccountInfo,
        to: AccountInfo,
        amount: u64,
        min_rent_lamports: u64,
    ) -> Result<()> {
        // debit lamports
        let pre_lamports = from.lamports();
        **from.try_borrow_mut_lamports()? = pre_lamports.checked_sub(amount).expect(&*format!(
            "debit lamports overflow: [from: {}, pre_lamports: {}, amount: {}]",
            from.key(),
            pre_lamports,
            amount,
        ));
        if from.lamports() < min_rent_lamports {
            return Err(RentExemptViolation.into());
        }

        // credit lamports
        let pre_lamports = to.lamports();
        **to.try_borrow_mut_lamports()? = pre_lamports.checked_add(amount).expect(&*format!(
            "credit lamports overflow: [to: {}, pre_lamports: {}, amount: {}]",
            to.key(),
            pre_lamports,
            amount,
        ));

        Ok(())
    }
}

/// Gives us an audit trail of who and what was claimed; also enforces and only-once claim by any party.
#[account]
#[derive(Default)]
pub struct ClaimStatus {
    /// If true, the tokens have been claimed.
    pub is_claimed: bool,

    /// Authority that claimed the tokens. Allows for delegated rewards claiming.
    pub claimant: Pubkey,

    /// The payer who created the claim.
    pub claim_status_payer: Pubkey,

    /// When the funds were claimed.
    pub slot_claimed_at: u64,

    /// Amount of funds claimed.
    pub amount: u64,

    /// The epoch (upto and including) that tip funds can be claimed.
    /// Copied since TDA can be closed, need to track to avoid making multiple claims
    pub expires_at: u64,

    /// The bump used to generate this account
    pub bump: u8,
}

impl ClaimStatus {
    pub const SEED: &'static [u8] = b"CLAIM_STATUS";

    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();
}
