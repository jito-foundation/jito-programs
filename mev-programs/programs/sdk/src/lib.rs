pub mod instructions;

use anchor_lang::{prelude::Pubkey, solana_program::clock::Epoch};
use jito_tip_distribution::state::{Config, TipDistributionAccount};

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

pub fn derive_config_account_address(tip_distribution_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[Config::SEED], tip_distribution_program_id)
}
