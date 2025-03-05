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
        data: jito_tip_distribution::instruction::Initialize {
            authority,
            expired_funds_account,
            num_epochs_valid,
            max_validator_commission_bps,
            bump,
        }
        .data(),
        accounts: jito_tip_distribution::accounts::Initialize {
            config,
            system_program,
            initializer,
        }
        .to_account_metas(None),
    }
}

pub struct InitializeTipDistributionAccountArgs {
    pub merkle_root_upload_authority: Pubkey,
    pub validator_commission_bps: u16,
    pub bump: u8,
}
pub struct InitializeTipDistributionAccountAccounts {
    pub config: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
    pub tip_distribution_account: Pubkey,
    pub validator_vote_account: Pubkey,
}
pub fn initialize_tip_distribution_account_ix(
    program_id: Pubkey,
    args: InitializeTipDistributionAccountArgs,
    accounts: InitializeTipDistributionAccountAccounts,
) -> Instruction {
    let InitializeTipDistributionAccountArgs {
        merkle_root_upload_authority,
        validator_commission_bps,
        bump,
    } = args;

    let InitializeTipDistributionAccountAccounts {
        config,
        tip_distribution_account,
        system_program,
        validator_vote_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: jito_tip_distribution::instruction::InitializeTipDistributionAccount {
            merkle_root_upload_authority,
            validator_commission_bps,
            bump,
        }
        .data(),
        accounts: jito_tip_distribution::accounts::InitializeTipDistributionAccount {
            config,
            signer,
            system_program,
            tip_distribution_account,
            validator_vote_account,
        }
        .to_account_metas(None),
    }
}

pub struct InitializeMerkleRootUploadConfigArgs {
    pub override_authority: Pubkey,
    pub original_authority: Pubkey,
}
pub struct InitializeMerkleRootUploadConfigAccounts {
    pub authority: Pubkey,
    pub config: Pubkey,
    pub merkle_root_upload_config: Pubkey,
    pub payer: Pubkey,
    pub system_program: Pubkey,
}
pub fn initialize_merkle_root_upload_config_ix(
    program_id: Pubkey,
    args: InitializeMerkleRootUploadConfigArgs,
    accounts: InitializeMerkleRootUploadConfigAccounts,
) -> Instruction {
    let InitializeMerkleRootUploadConfigArgs {
        override_authority,
        original_authority,
    } = args;

    let InitializeMerkleRootUploadConfigAccounts {
        authority,
        config,
        merkle_root_upload_config,
        payer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: jito_tip_distribution::instruction::InitializeMerkleRootUploadConfig {
            authority: override_authority,
            original_authority,
        }
        .data(),
        accounts: jito_tip_distribution::accounts::InitializeMerkleRootUploadConfig {
            authority,
            config,
            merkle_root_upload_config,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateMerkleRootUploadConfigArgs {
    pub authority: Pubkey,
    pub original_authority: Pubkey,
}
pub struct UpdateMerkleRootUploadConfigAccounts {
    pub authority: Pubkey,
    pub config: Pubkey,
    pub merkle_root_upload_config: Pubkey,
    pub system_program: Pubkey,
}

pub fn update_merkle_root_upload_config_ix(
    program_id: Pubkey,
    args: UpdateMerkleRootUploadConfigArgs,
    accounts: UpdateMerkleRootUploadConfigAccounts,
) -> Instruction {
    let UpdateMerkleRootUploadConfigArgs {
        authority: _,
        original_authority,
    } = args;

    let UpdateMerkleRootUploadConfigAccounts {
        authority,
        config,
        merkle_root_upload_config,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: jito_tip_distribution::instruction::UpdateMerkleRootUploadConfig {
            authority,
            original_authority,
        }
        .data(),
        accounts: jito_tip_distribution::accounts::UpdateMerkleRootUploadConfig {
            authority,
            config,
            merkle_root_upload_config,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct CloseClaimStatusArgs;
pub struct CloseClaimStatusAccounts {
    pub config: Pubkey,
    pub claim_status: Pubkey,
    pub claim_status_payer: Pubkey,
}
pub fn close_claim_status_ix(
    program_id: Pubkey,
    _args: CloseClaimStatusArgs,
    accounts: CloseClaimStatusAccounts,
) -> Instruction {
    let CloseClaimStatusAccounts {
        config,
        claim_status,
        claim_status_payer,
    } = accounts;

    Instruction {
        program_id,
        data: jito_tip_distribution::instruction::CloseClaimStatus {}.data(),
        accounts: jito_tip_distribution::accounts::CloseClaimStatus {
            config,
            claim_status,
            claim_status_payer,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateConfigArgs {
    pub new_config: Config,
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
        data: jito_tip_distribution::instruction::UpdateConfig { new_config }.data(),
        accounts: jito_tip_distribution::accounts::UpdateConfig { config, authority }
            .to_account_metas(None),
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
        data: jito_tip_distribution::instruction::UploadMerkleRoot {
            max_total_claim,
            max_num_nodes,
            root,
        }
        .data(),
        accounts: jito_tip_distribution::accounts::UploadMerkleRoot {
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
pub struct CloseTipDistributionAccounts {
    pub config: Pubkey,
    pub tip_distribution_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub expired_funds_account: Pubkey,
    pub signer: Pubkey,
}
pub fn close_tip_distribution_account_ix(
    program_id: Pubkey,
    args: CloseTipDistributionAccountArgs,
    accounts: CloseTipDistributionAccounts,
) -> Instruction {
    let CloseTipDistributionAccountArgs { _epoch } = args;

    let CloseTipDistributionAccounts {
        config,
        tip_distribution_account,
        validator_vote_account,
        expired_funds_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: jito_tip_distribution::instruction::CloseTipDistributionAccount { _epoch }.data(),
        accounts: jito_tip_distribution::accounts::CloseTipDistributionAccount {
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
    pub bump: u8,
}
pub struct ClaimAccounts {
    pub config: Pubkey,
    pub tip_distribution_account: Pubkey,
    pub merkle_root_upload_authority: Pubkey,
    pub claim_status: Pubkey,
    pub claimant: Pubkey,
    pub payer: Pubkey,
    pub system_program: Pubkey,
}
pub fn claim_ix(program_id: Pubkey, args: ClaimArgs, accounts: ClaimAccounts) -> Instruction {
    let ClaimArgs {
        proof,
        amount,
        bump,
    } = args;

    let ClaimAccounts {
        config,
        tip_distribution_account,
        merkle_root_upload_authority,
        claim_status,
        claimant,
        payer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: jito_tip_distribution::instruction::Claim {
            proof,
            amount,
            bump,
        }
        .data(),
        accounts: jito_tip_distribution::accounts::Claim {
            config,
            tip_distribution_account,
            merkle_root_upload_authority,
            claimant,
            claim_status,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}
