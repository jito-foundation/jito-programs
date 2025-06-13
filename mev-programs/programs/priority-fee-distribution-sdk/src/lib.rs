use anchor_lang::prelude::Pubkey;
use jito_priority_fee_distribution::state::{Config, PriorityFeeDistributionAccount};

pub fn derive_priority_fee_distribution_account_address(
    priority_fee_distribution_program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    epoch: u64,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            PriorityFeeDistributionAccount::SEED,
            vote_pubkey.to_bytes().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        priority_fee_distribution_program_id,
    )
}

pub fn derive_config_account_address(
    priority_fee_distribution_program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[Config::SEED], priority_fee_distribution_program_id)
}
