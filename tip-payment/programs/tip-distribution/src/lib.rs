pub mod merkle_proof;
pub mod sdk;
pub mod state;

use anchor_lang::prelude::*;

use crate::{
    state::{ClaimStatus, Config, MerkleRoot, TipDistributionAccount},
    ErrorCode::Unauthorized,
};

declare_id!("7QnFbRZajkym8mUh9rXuM5nKrPAPRfEU6W31izSWJDVh");

#[program]
pub mod tip_distribution {
    use super::*;
    use crate::ErrorCode::{
        ExceedsMaxClaim, ExceedsMaxNumNodes, ExpiredTipDistributionAccount, FundsAlreadyClaimed,
        InvalidProof, MaxValidatorCommissionFeeBpsExceeded, PrematureCloseTipDistributionAccount,
        PrematureMerkleRootUpload, RootNotUploaded, Unauthorized,
    };

    /// Initialize a singleton instance of the [Config] account.
    pub fn initialize(
        ctx: Context<Initialize>,
        authority: Pubkey,
        expired_funds_account: Pubkey,
        num_epochs_valid: u64,
        max_validator_commission_bps: u16,
        bump: u8,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.authority = authority;
        cfg.expired_funds_account = expired_funds_account;
        cfg.num_epochs_valid = num_epochs_valid;
        cfg.max_validator_commission_bps = max_validator_commission_bps;
        cfg.bump = bump;
        cfg.validate()?;

        Ok(())
    }

    /// Initialize a new [TipDistributionAccount] associated with the given validator vote key
    /// and current epoch.
    pub fn init_tip_distribution_account(
        ctx: Context<InitTipDistributionAccount>,
        merkle_root_upload_authority: Pubkey,
        validator_vote_account: Pubkey,
        validator_commission_bps: u16,
        bump: u8,
    ) -> Result<()> {
        if validator_commission_bps > ctx.accounts.config.max_validator_commission_bps {
            return Err(MaxValidatorCommissionFeeBpsExceeded.into());
        }

        let distribution_acc = &mut ctx.accounts.tip_distribution_account;
        distribution_acc.validator_vote_account = validator_vote_account;
        distribution_acc.epoch_created_at = Clock::get()?.epoch;
        distribution_acc.validator_commission_bps = validator_commission_bps;
        distribution_acc.merkle_root_upload_authority = merkle_root_upload_authority;
        distribution_acc.merkle_root = None;
        distribution_acc.bump = bump;
        distribution_acc.validate()?;

        emit!(TipDistributionAccountInitializedEvent {
            tip_distribution_account: distribution_acc.key(),
        });

        Ok(())
    }

    /// Sets a new `validator_commission_bps` rate for the given [TipDistributionAccount]. Only the
    /// associated validator can invoke this instruction.
    pub fn set_validator_commission_bps(
        ctx: Context<SetValidatorCommissionBps>,
        new_validator_commission_bps: u16,
    ) -> Result<()> {
        SetValidatorCommissionBps::auth(&ctx)?;

        let distribution_acc = &mut ctx.accounts.tip_distribution_account;
        if new_validator_commission_bps > ctx.accounts.config.max_validator_commission_bps {
            return Err(MaxValidatorCommissionFeeBpsExceeded.into());
        }

        let old_commission_bps = distribution_acc.validator_commission_bps;
        distribution_acc.validator_commission_bps = new_validator_commission_bps;
        distribution_acc.validate()?;

        emit!(ValidatorCommissionBpsUpdatedEvent {
            tip_distribution_account: distribution_acc.key(),
            old_commission_bps,
            new_commission_bps: new_validator_commission_bps,
        });

        Ok(())
    }

    /// Sets a new `merkle_root_upload_authority` for the given [TipDistributionAccount]. Only the
    /// associated validator can invoke this instruction.
    pub fn set_merkle_root_upload_authority(
        ctx: Context<SetMerkleRootUploadAuthority>,
        new_merkle_root_upload_authority: Pubkey,
    ) -> Result<()> {
        SetMerkleRootUploadAuthority::auth(&ctx)?;

        let distribution_acc = &mut ctx.accounts.tip_distribution_account;
        if let Some(merkle_root) = &distribution_acc.merkle_root {
            if merkle_root.num_nodes_claimed > 0 {
                return Err(Unauthorized.into());
            }
        }

        let old_authority = distribution_acc.merkle_root_upload_authority;
        distribution_acc.merkle_root_upload_authority = new_merkle_root_upload_authority;
        distribution_acc.validate()?;

        emit!(MerkleRootUploadAuthorityUpdatedEvent {
            old_authority,
            new_authority: new_merkle_root_upload_authority
        });

        Ok(())
    }

    /// Update config fields. Only the [Config] authority can invoke this.
    pub fn update_config(ctx: Context<UpdateConfig>, new_config: Config) -> Result<()> {
        UpdateConfig::auth(&ctx)?;

        let config = &mut ctx.accounts.config;
        config.authority = new_config.authority;
        config.expired_funds_account = new_config.expired_funds_account;
        config.num_epochs_valid = new_config.num_epochs_valid;
        config.max_validator_commission_bps = new_config.max_validator_commission_bps;
        config.validate()?;

        emit!(ConfigUpdatedEvent {
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    /// Uploads a merkle root to the provided [TipDistributionAccount]. This instruction may be
    /// invoked many times as long as the account is at least one epoch old and not expired; and
    /// no funds have already been claimed. Only the `merkle_root_upload_authority` has the
    /// authority to invoke.
    pub fn upload_merkle_root(
        ctx: Context<UploadMerkleRoot>,
        root: [u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
    ) -> Result<()> {
        UploadMerkleRoot::auth(&ctx)?;

        let current_epoch = Clock::get().unwrap().epoch;
        let distribution_acc = &mut ctx.accounts.tip_distribution_account;

        if let Some(merkle_root) = &distribution_acc.merkle_root {
            if merkle_root.num_nodes_claimed > 0 {
                return Err(Unauthorized.into());
            }
        }
        if distribution_acc.is_expired(&ctx.accounts.config, current_epoch) {
            return Err(ExpiredTipDistributionAccount.into());
        }
        if distribution_acc.epoch_created_at >= current_epoch {
            return Err(PrematureMerkleRootUpload.into());
        }

        distribution_acc.merkle_root = Some(MerkleRoot {
            root,
            max_total_claim,
            max_num_nodes,
            total_funds_claimed: 0,
            num_nodes_claimed: 0,
        });
        distribution_acc.validate()?;

        emit!(MerkleRootUploadedEvent {
            merkle_root_upload_authority: ctx.accounts.merkle_root_upload_authority.key(),
            tip_distribution_account: distribution_acc.key(),
        });

        Ok(())
    }

    /// Anyone can invoke this only after the [TipDistributionAccount] has expired.
    /// This instruction will send any unclaimed funds to the designated `expired_funds_account`
    /// before closing and returning the rent exempt funds to the validator.
    pub fn close_tip_distribution_account(
        ctx: Context<CloseTipDistributionAccount>,
        _epoch: u64,
    ) -> Result<()> {
        CloseTipDistributionAccount::auth(&ctx)?;

        let current_epoch = Clock::get().unwrap().epoch;

        let tip_distribution_account = &mut ctx.accounts.tip_distribution_account;
        if tip_distribution_account.is_expired(&ctx.accounts.config, current_epoch) {
            let expired_amount = TipDistributionAccount::claim_expired(
                tip_distribution_account.to_account_info(),
                ctx.accounts.expired_funds_account.to_account_info(),
            )?;
            tip_distribution_account.validate()?;

            emit!(TipDistributionAccountClosedEvent {
                expired_funds_account: ctx.accounts.expired_funds_account.key(),
                tip_distribution_account: tip_distribution_account.key(),
                expired_amount,
            });

            Ok(())
        } else {
            Err(PrematureCloseTipDistributionAccount.into())
        }
    }

    /// Claims tokens from the [TipDistributionAccount].
    pub fn claim(ctx: Context<Claim>, bump: u8, amount: u64, proof: Vec<[u8; 32]>) -> Result<()> {
        let claim_status = &mut ctx.accounts.claim_status;
        claim_status.bump = bump;

        let claimant_account = &mut ctx.accounts.claimant;
        let tip_distribution_account = &mut ctx.accounts.tip_distribution_account;
        let clock = Clock::get()?;

        // Redundant check since we shouldn't be able to init a claim status account using the same seeds.
        if claim_status.is_claimed {
            return Err(FundsAlreadyClaimed.into());
        }

        let tip_distribution_info = tip_distribution_account.to_account_info();
        let merkle_root = tip_distribution_account
            .merkle_root
            .as_mut()
            .ok_or(RootNotUploaded)?;

        // Verify the merkle proof.
        let node = &solana_program::hash::hashv(&[
            &[0u8],
            &solana_program::hash::hashv(&[
                &claimant_account.key().to_bytes(),
                &amount.to_le_bytes(),
            ])
            .to_bytes(),
        ]);

        if !merkle_proof::verify(proof, merkle_root.root, node.to_bytes()) {
            return Err(InvalidProof.into());
        }

        TipDistributionAccount::claim(
            tip_distribution_info,
            claimant_account.to_account_info(),
            amount,
        )?;

        // Mark it claimed.
        claim_status.amount = amount;
        claim_status.is_claimed = true;
        claim_status.slot_claimed_at = clock.slot;
        claim_status.claimant = claimant_account.key();

        merkle_root.total_funds_claimed.checked_add(amount).unwrap();
        if merkle_root.total_funds_claimed > merkle_root.max_total_claim {
            return Err(ExceedsMaxClaim.into());
        }
        merkle_root.num_nodes_claimed.checked_add(1).unwrap();
        if merkle_root.num_nodes_claimed > merkle_root.max_num_nodes {
            return Err(ExceedsMaxNumNodes.into());
        }

        emit!(ClaimedEvent {
            tip_distribution_account: tip_distribution_account.key(),
            payer: ctx.accounts.payer.key(),
            claimant: claimant_account.key(),
            amount
        });

        tip_distribution_account.validate()?;

        Ok(())
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Account failed validation.")]
    AccountValidationFailure,

    #[msg("The maximum number of funds to be claimed has been exceeded.")]
    ExceedsMaxClaim,

    #[msg("The maximum number of claims has been exceeded.")]
    ExceedsMaxNumNodes,

    #[msg("The given TipDistributionAccount has expired.")]
    ExpiredTipDistributionAccount,

    #[msg("The funds for the given index and TipDistributionAccount have already been claimed.")]
    FundsAlreadyClaimed,

    #[msg("The given proof is invalid.")]
    InvalidProof,

    #[msg("Validator's commission basis points must be less than or equal to the Config account's max_validator_commission_bps.")]
    MaxValidatorCommissionFeeBpsExceeded,

    #[msg("The given TipDistributionAccount is not ready to be closed.")]
    PrematureCloseTipDistributionAccount,

    #[msg("Must wait till at least one epoch after the tip distribution account was created to upload the merkle root.")]
    PrematureMerkleRootUpload,

    #[msg("Account would violate rent exemption.")]
    RentExemptViolation,

    #[msg("No merkle root has been uploaded to the given TipDistributionAccount.")]
    RootNotUploaded,

    #[msg("Unauthorized signer.")]
    Unauthorized,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [Config::SEED],
        bump,
        payer = initializer,
        space = Config::SIZE,
        rent_exempt = enforce
    )]
    pub config: Account<'info, Config>,

    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub initializer: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(
    _merkle_root_upload_authority: Pubkey,
    validator_vote_account: Pubkey,
    _validator_commission_bps: u16,
    _bump: u8
)]
pub struct InitTipDistributionAccount<'info> {
    pub config: Account<'info, Config>,

    #[account(
        init,
        seeds = [
            TipDistributionAccount::SEED,
            validator_vote_account.as_ref(),
            Clock::get().unwrap().epoch.to_le_bytes().as_ref(),
        ],
        bump,
        payer = payer,
        space = TipDistributionAccount::SIZE,
        rent_exempt = enforce
    )]
    pub tip_distribution_account: Account<'info, TipDistributionAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetMerkleRootUploadAuthority<'info> {
    #[account(
        mut,
        rent_exempt = enforce
    )]
    pub tip_distribution_account: Account<'info, TipDistributionAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

impl SetMerkleRootUploadAuthority<'_> {
    fn auth(ctx: &Context<SetMerkleRootUploadAuthority>) -> Result<()> {
        if ctx.accounts.tip_distribution_account.validator_vote_account != ctx.accounts.signer.key()
        {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

#[derive(Accounts)]
pub struct SetValidatorCommissionBps<'info> {
    pub config: Account<'info, Config>,

    #[account(
        mut,
        rent_exempt = enforce
    )]
    pub tip_distribution_account: Account<'info, TipDistributionAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

impl SetValidatorCommissionBps<'_> {
    fn auth(ctx: &Context<SetValidatorCommissionBps>) -> Result<()> {
        if ctx.accounts.tip_distribution_account.validator_vote_account != ctx.accounts.signer.key()
        {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut, rent_exempt = enforce)]
    pub config: Account<'info, Config>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

impl UpdateConfig<'_> {
    fn auth(ctx: &Context<UpdateConfig>) -> Result<()> {
        if ctx.accounts.config.authority != ctx.accounts.authority.key() {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct CloseTipDistributionAccount<'info> {
    pub config: Account<'info, Config>,

    /// CHECK: safe see auth fn
    #[account(mut)]
    pub expired_funds_account: AccountInfo<'info>,

    #[account(
        mut,
        close = validator_vote_account,
        seeds = [
            TipDistributionAccount::SEED,
            validator_vote_account.key().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        bump = tip_distribution_account.bump,
    )]
    pub tip_distribution_account: Account<'info, TipDistributionAccount>,

    /// CHECK: safe see auth fn
    #[account(mut)]
    pub validator_vote_account: AccountInfo<'info>,

    /// Anyone can crank this instruction.
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl CloseTipDistributionAccount<'_> {
    fn auth(ctx: &Context<CloseTipDistributionAccount>) -> Result<()> {
        if ctx.accounts.config.expired_funds_account != ctx.accounts.expired_funds_account.key() {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

#[derive(Accounts)]
#[instruction(_bump: u8, _amount: u64, _proof: Vec<[u8; 32]>)]
pub struct Claim<'info> {
    pub config: Account<'info, Config>,

    #[account(mut, rent_exempt = enforce)]
    pub tip_distribution_account: Account<'info, TipDistributionAccount>,

    /// Status of the claim. Used to prevent the same party from claiming multiple times.
    #[account(
        init,
        rent_exempt = enforce,
        seeds = [
            ClaimStatus::SEED,
            claimant.key().as_ref(),
            tip_distribution_account.key().as_ref()
        ],
        bump,
        space = ClaimStatus::SIZE,
        payer = payer
    )]
    pub claim_status: Account<'info, ClaimStatus>,

    /// CHECK: This is safe.
    /// Receiver of the funds.
    #[account(mut)]
    pub claimant: AccountInfo<'info>,

    /// Who is paying for the claim.
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UploadMerkleRoot<'info> {
    pub config: Account<'info, Config>,

    #[account(mut, rent_exempt = enforce)]
    pub tip_distribution_account: Account<'info, TipDistributionAccount>,

    #[account(mut)]
    pub merkle_root_upload_authority: Signer<'info>,
}

impl UploadMerkleRoot<'_> {
    fn auth(ctx: &Context<UploadMerkleRoot>) -> Result<()> {
        if ctx.accounts.merkle_root_upload_authority.key()
            != ctx
                .accounts
                .tip_distribution_account
                .merkle_root_upload_authority
        {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

// Events

#[event]
pub struct TipDistributionAccountInitializedEvent {
    pub tip_distribution_account: Pubkey,
}

#[event]
pub struct ValidatorCommissionBpsUpdatedEvent {
    pub tip_distribution_account: Pubkey,
    pub old_commission_bps: u16,
    pub new_commission_bps: u16,
}

#[event]
pub struct MerkleRootUploadAuthorityUpdatedEvent {
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
}

#[event]
pub struct ConfigUpdatedEvent {
    /// Who updated it.
    authority: Pubkey,
}

#[event]
pub struct ClaimedEvent {
    /// [TipDistributionAccount] claimed from.
    pub tip_distribution_account: Pubkey,

    /// User that paid for the claim, may or may not be the same as claimant.
    pub payer: Pubkey,

    /// Account that received the funds.
    pub claimant: Pubkey,

    /// Amount of funds to distribute.
    pub amount: u64,
}

#[event]
pub struct MerkleRootUploadedEvent {
    /// Who uploaded the root.
    pub merkle_root_upload_authority: Pubkey,

    /// Where the root was uploaded to.
    pub tip_distribution_account: Pubkey,
}

#[event]
pub struct TipDistributionAccountClosedEvent {
    /// Account where unclaimed funds were transferred to.
    pub expired_funds_account: Pubkey,

    /// [TipDistributionAccount] closed.
    pub tip_distribution_account: Pubkey,

    /// Unclaimed amount transferred.
    pub expired_amount: u64,
}
