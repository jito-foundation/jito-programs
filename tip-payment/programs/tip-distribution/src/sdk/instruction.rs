//! This module contains functions that build instructions to interact with the tip-distribution program.
use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::Config;

pub struct InitializeArgs {
    pub authority: Pubkey,
    pub expired_funds_account: Pubkey,
    pub num_epochs_valid: u64,
    pub max_validator_commission_bps: u16,
    pub bump: u8,
}
pub struct InitializeAccounts {
    pub config: Pubkey,
    pub system_program: Pubkey,
    pub initializer: Pubkey,
}
pub fn initialize_ix(
    program_id: Pubkey,
    args: InitializeArgs,
    accounts: InitializeAccounts,
) -> Instruction {
    let InitializeArgs {
        authority,
        expired_funds_account,
        num_epochs_valid,
        max_validator_commission_bps,
        bump,
    } = args;

    let InitializeAccounts {
        config,
        system_program,
        initializer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::Initialize {
            authority,
            expired_funds_account,
            num_epochs_valid,
            max_validator_commission_bps,
            bump,
        }
        .data(),
        accounts: crate::accounts::Initialize {
            config,
            system_program,
            initializer,
        }
        .to_account_metas(None),
    }
}

pub struct InitTipDistributionAccountArgs {
    pub merkle_root_upload_authority: Pubkey,
    pub validator_commission_bps: u16,
    pub bump: u8,
}
pub struct InitTipDistributionAccountAccounts {
    pub config: Pubkey,
    pub tip_distribution_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub system_program: Pubkey,
}
pub fn init_tip_distribution_account_ix(
    program_id: Pubkey,
    args: InitTipDistributionAccountArgs,
    accounts: InitTipDistributionAccountAccounts,
) -> Instruction {
    let InitTipDistributionAccountArgs {
        merkle_root_upload_authority,
        validator_commission_bps,
        bump,
    } = args;

    let InitTipDistributionAccountAccounts {
        config,
        tip_distribution_account,
        validator_vote_account,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitTipDistributionAccount {
            merkle_root_upload_authority,
            validator_commission_bps,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitTipDistributionAccount {
            config,
            tip_distribution_account,
            validator_vote_account,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct SetValidatorCommissionBpsArgs {
    pub new_validator_commission_bps: u16,
}
pub struct SetValidatorCommissionBpsAccounts {
    pub config: Pubkey,
    pub tip_distribution_account: Pubkey,
    pub validator_vote_account: Pubkey,
}
pub fn set_validator_commission_bps_ix(
    program_id: Pubkey,
    args: SetValidatorCommissionBpsArgs,
    accounts: SetValidatorCommissionBpsAccounts,
) -> Instruction {
    let SetValidatorCommissionBpsArgs {
        new_validator_commission_bps,
    } = args;

    let SetValidatorCommissionBpsAccounts {
        config,
        tip_distribution_account,
        validator_vote_account,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::SetValidatorCommissionBps {
            new_validator_commission_bps,
        }
        .data(),
        accounts: crate::accounts::SetValidatorCommissionBps {
            config,
            tip_distribution_account,
            validator_vote_account,
        }
        .to_account_metas(None),
    }
}

pub struct SetMerkleRootUploadAuthorityArgs {
    pub new_merkle_root_upload_authority: Pubkey,
}
pub struct SetMerkleRootUploadAuthorityAccounts {
    pub tip_distribution_account: Pubkey,
    pub validator_vote_account: Pubkey,
}
pub fn set_merkle_root_upload_authority_ix(
    program_id: Pubkey,
    args: SetMerkleRootUploadAuthorityArgs,
    accounts: SetMerkleRootUploadAuthorityAccounts,
) -> Instruction {
    let SetMerkleRootUploadAuthorityArgs {
        new_merkle_root_upload_authority,
    } = args;

    let SetMerkleRootUploadAuthorityAccounts {
        tip_distribution_account,
        validator_vote_account,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::SetMerkleRootUploadAuthority {
            new_merkle_root_upload_authority,
        }
        .data(),
        accounts: crate::accounts::SetMerkleRootUploadAuthority {
            tip_distribution_account,
            validator_vote_account,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateConfigArgs {
    new_config: Config,
}
pub struct UpdateConfigAccounts {
    pub config: Pubkey,
    pub authority: Pubkey,
}
pub fn update_config_ix(
    program_id: Pubkey,
    args: UpdateConfigArgs,
    accounts: UpdateConfigAccounts,
) -> Instruction {
    let UpdateConfigArgs { new_config } = args;

    let UpdateConfigAccounts { config, authority } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateConfig { new_config }.data(),
        accounts: crate::accounts::UpdateConfig { config, authority }.to_account_metas(None),
    }
}

pub struct UploadMerkleRootArgs {
    pub root: [u8; 32],
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
}
pub struct UploadMerkleRootAccounts {
    pub config: Pubkey,
    pub merkle_root_upload_authority: Pubkey,
    pub tip_distribution_account: Pubkey,
}
pub fn upload_merkle_root_ix(
    program_id: Pubkey,
    args: UploadMerkleRootArgs,
    accounts: UploadMerkleRootAccounts,
) -> Instruction {
    let UploadMerkleRootArgs {
        root,
        max_total_claim,
        max_num_nodes,
    } = args;

    let UploadMerkleRootAccounts {
        config,
        merkle_root_upload_authority,
        tip_distribution_account,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UploadMerkleRoot {
            max_total_claim,
            max_num_nodes,
            root,
        }
        .data(),
        accounts: crate::accounts::UploadMerkleRoot {
            config,
            merkle_root_upload_authority,
            tip_distribution_account,
        }
        .to_account_metas(None),
    }
}

pub struct CloseTipDistributionAccountArgs {
    pub _epoch: u64,
}
pub struct CloseTipDistributionAccount {
    pub config: Pubkey,
    pub tip_distribution_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub expired_funds_account: Pubkey,
    pub signer: Pubkey,
}
pub fn close_tip_distribution_account_ix(
    program_id: Pubkey,
    args: CloseTipDistributionAccountArgs,
    accounts: CloseTipDistributionAccount,
) -> Instruction {
    let CloseTipDistributionAccountArgs { _epoch } = args;

    let CloseTipDistributionAccount {
        config,
        tip_distribution_account,
        validator_vote_account,
        expired_funds_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseTipDistributionAccount { _epoch }.data(),
        accounts: crate::accounts::CloseTipDistributionAccount {
            config,
            validator_vote_account,
            expired_funds_account,
            tip_distribution_account,
            signer,
        }
        .to_account_metas(None),
    }
}

pub struct ClaimArgs {
    pub proof: Vec<[u8; 32]>,
    pub amount: u64,
    pub index: u64,
    pub bump: u8,
}
pub struct ClaimAccounts {
    pub config: Pubkey,
    pub tip_distribution_account: Pubkey,
    pub claim_status: Pubkey,
    pub claimant: Pubkey,
    pub payer: Pubkey,
    pub system_program: Pubkey,
}
pub fn claim_ix(program_id: Pubkey, args: ClaimArgs, accounts: ClaimAccounts) -> Instruction {
    let ClaimArgs {
        proof,
        amount,
        index,
        bump,
    } = args;

    let ClaimAccounts {
        config,
        tip_distribution_account,
        claim_status,
        claimant,
        payer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::Claim {
            proof,
            amount,
            index,
            bump,
        }
        .data(),
        accounts: crate::accounts::Claim {
            config,
            tip_distribution_account,
            claimant,
            claim_status,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}
