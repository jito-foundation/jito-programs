/// This module is where all PDA structs lives.
use std::mem::size_of;

use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct Config {
    /// This account has access to admin-like instructions such as those that update [Config] fields
    /// as well as those that debit funds from the mev_distribution accounts.
    pub authority: Pubkey,
    /// Account which all `mev_distribution` account funds will be transferred to each epoch.
    /// Store this field for increased transparency into how funds are transferred each epoch.
    /// Without this users would have to *trust* that the correct `to` address is being passed to
    /// the `TransferDistributionAccountFunds` instruction.
    pub distribution_pot: Pubkey,
    /// The maximum `payer_fee_split_bps` `mev_distribution` account payers (validators) can set.
    pub max_payer_fee_bps: u16,
}

/// [MevDistributionAccount] accounts are PDAs using the account payer's public key and current epoch as seeds.
#[account]
#[derive(Default)]
pub struct MevDistributionAccount {
    /// Account that paid for the initialization of this account.
    pub payer: Pubkey,
    /// Epoch for which this account was created.  
    pub epoch_created: u64,
    /// The fee percentage that will be distributed to payers (validators) at the end of the epoch.
    pub payer_fee_split_bps: u16,
    // TODO might not be necessary
    pub bump: u8,
}

const HEADER_SIZE: usize = 8;

impl MevDistributionAccount {
    pub const SEED: &'static [u8] = b"MEV_DISTRIBUTION_ACCOUNT";
    /// URL sizes can vary in length, therefore we subtract `size_of::<String>()`, corresponding to
    /// the `backend_url` field. Payer is expected to supply the `init` attribute with the extra
    /// padding required for this field.
    pub const SIZE: usize = HEADER_SIZE + size_of::<Self>();
}
