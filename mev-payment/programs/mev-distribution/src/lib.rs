mod state;

use anchor_lang::prelude::*;

use crate::state::{Config, MevDistributionAccount};

declare_id!("BsnxG8jiKuQxUegpkgcGx3EHs6GD7KrZJjBy53a1Ybaa");

#[program]
pub mod mev_distribution {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        authority: Pubkey,
        distribution_pot: Pubkey,
        max_payer_fee_bps: u16,
        _config_bump: u8,
    ) -> ProgramResult {
        let cfg = &mut ctx.accounts.config;
        cfg.authority = authority;
        cfg.distribution_pot = distribution_pot;
        cfg.max_payer_fee_bps = max_payer_fee_bps;

        Ok(())
    }

    pub fn init_distribution_account(
        ctx: Context<InitDistributionAccount>,
        payer_fee_split_bps: u16,
        bump: u8,
    ) -> ProgramResult {
        if payer_fee_split_bps > ctx.accounts.config.max_payer_fee_bps {
            return Err(ErrorCode::InvalidValidatorFeeSplitBps.into());
        }
        let distribution_acc = &mut ctx.accounts.distribution_account;
        distribution_acc.payer = ctx.accounts.initializer.key();
        distribution_acc.epoch_created = Clock::get()?.epoch;
        distribution_acc.payer_fee_split_bps = payer_fee_split_bps;
        distribution_acc.bump = bump;

        Ok(())
    }

    pub fn transfer_distribution_account_funds(
        ctx: Context<TransferDistributionAccountFunds>,
    ) -> ProgramResult {
        check_authority(&ctx.accounts.authority, &ctx.accounts.config)?;
        let from = ctx.accounts.from.to_account_info();
        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(MevDistributionAccount::SIZE);
        let lamports = from.lamports().checked_sub(min_rent).expect(&*format!(
            "lamports calc overflow: [from: {}, lamports: {}, min_rent: {}]",
            from.key(),
            from.lamports(),
            min_rent,
        ));
        // debit lamports
        let pre_lamports = from.lamports();
        **from.try_borrow_mut_lamports()? = pre_lamports.checked_sub(lamports).expect(&*format!(
            "debit lamports overflow: [from: {}, pre_lamports: {}, lamports: {}]",
            from.key(),
            pre_lamports,
            lamports,
        ));
        // credit lamports
        let pre_lamports = ctx.accounts.to.lamports();
        **ctx.accounts.to.try_borrow_mut_lamports()? =
            pre_lamports.checked_add(lamports).expect(&*format!(
                "credit lamports overflow: [to: {}, pre_lamports: {}, lamports: {}]",
                ctx.accounts.to.key(),
                pre_lamports,
                lamports,
            ));

        Ok(())
    }
}

fn check_authority(signer: &Signer, config: &Account<Config>) -> ProgramResult {
    if signer.key != &config.authority {
        return Err(ErrorCode::Unauthorized.into());
    }
    Ok(())
}

#[error]
pub enum ErrorCode {
    #[msg("Validator's fee split basis points must less than or equal to max_validator_fee_bps")]
    InvalidValidatorFeeSplitBps,
    #[msg("Signer not authorized to perform this action.")]
    Unauthorized,
}

const CONFIG_ACCOUNT_SEED: &[u8] = b"CONFIG_ACCOUNT";

#[derive(Accounts)]
#[instruction(authority: Pubkey, distribution_pot: Pubkey, max_payer_fee_bps: u16, bump: u8)]
pub struct Initialize<'info> {
    /// singleton account
    #[account(
        init,
        seeds = [CONFIG_ACCOUNT_SEED],
        bump = bump,
        payer = initializer,
    )]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub initializer: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_payer_fee_split_bps: u16, bump: u8)]
pub struct InitDistributionAccount<'info> {
    pub config: Account<'info, Config>,
    #[account(
        init,
        seeds = [
            MevDistributionAccount::SEED,
            initializer.key().as_ref(),
            Clock::get().unwrap().epoch.to_le_bytes().as_ref(),
        ],
        bump = bump,
        payer = initializer,
        space = MevDistributionAccount::SIZE
    )]
    pub distribution_account: Account<'info, MevDistributionAccount>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferDistributionAccountFunds<'info> {
    #[account(constraint = config.distribution_pot == to.key())]
    pub config: Account<'info, Config>,
    /// constraint is necessary to ensure the `close` target is the correct account
    #[account(
        mut,
        close = distribution_account_payer,
        constraint = from.payer == distribution_account_payer.key(),
    )]
    pub from: Account<'info, MevDistributionAccount>,
    #[account(mut)]
    pub to: AccountInfo<'info>,
    /// The account which was used as one of the seeds to derive the `mev_distribution` account.
    /// This account was also the payer so it will be refunded the rent exempt lamports.
    #[account(mut)]
    pub distribution_account_payer: AccountInfo<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
}
