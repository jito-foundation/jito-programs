use anchor_lang::prelude::*;
use std::mem::size_of;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

const CONFIG_ACCOUNT_SEED: &'static [u8] = b"CONFIG_ACCOUNT";

#[program]
pub mod payment_vault {
    use super::*;

    /// Can only be invoked once due to hardcoded Config account seed.
    pub fn initialize(ctx: Context<Initialize>, _config_account_bump: u8) -> ProgramResult {
        let cfg = &mut ctx.accounts.config;
        cfg.tip_claimer = ctx.accounts.initial_tip_claimer.key();
        Ok(())
    }

    #[access_control(auth_config_account(&ctx.accounts.config, ctx.program_id))]
    /// Validators should call this at the end of the slot or prior to rotation.
    /// If this isn't called before the next leader rotation, tips will be transferred
    /// in change_claimer before claimer is changed.
    pub fn claim_tips(ctx: Context<ClaimTips>) -> ProgramResult {
        let cfg_info = ctx.accounts.config.to_account_info();
        let tips = Config::calc_tips(cfg_info.lamports())?;

        if tips > 0 {
            // xfer tips to tip_claimer account
            **cfg_info.try_borrow_mut_lamports()? -= tips;
            **ctx.accounts.tip_claimer.try_borrow_mut_lamports()? += tips;
        }

        emit!(TipsClaimed {
            by: ctx.accounts.claimer.key(),
            to: ctx.accounts.tip_claimer.key(),
            amount: tips,
        });

        Ok(())
    }

    #[access_control(auth_config_account(&ctx.accounts.config, ctx.program_id))]
    /// Validator should include this at top of block, at beginning of rotation.
    pub fn change_tip_claimer(ctx: Context<ChangeTipClaimer>) -> ProgramResult {
        let cfg_info = ctx.accounts.config.to_account_info();
        let tips = Config::calc_tips(cfg_info.lamports())?;

        // if Config account has any tips, send to previous tip_claimer account
        if tips > 0 {
            msg!("claiming {} lamports to previous validator", tips);
            // drain account
            **cfg_info.try_borrow_mut_lamports()? -= tips;
            // move tips to old account
            **ctx.accounts.old_tip_claimer.try_borrow_mut_lamports()? += tips;

            emit!(TipsClaimed {
                by: ctx.accounts.signer.key(),
                to: ctx.accounts.old_tip_claimer.key(),
                amount: tips,
            });
        }

        // set new funding account
        ctx.accounts.config.tip_claimer = ctx.accounts.new_tip_claimer.key();

        emit!(TipClaimerChanged {
            new_tip_claimer: ctx.accounts.new_tip_claimer.key(),
            old_tip_claimer: ctx.accounts.old_tip_claimer.key(),
        });

        Ok(())
    }
}

fn auth_config_account(cfg: &Account<Config>, prog_id: &Pubkey) -> ProgramResult {
    let (pda, _bump) = Pubkey::find_program_address(&[&CONFIG_ACCOUNT_SEED], prog_id);
    let info = cfg.to_account_info();
    if *info.key != pda || info.owner != prog_id {
        return Err(ErrorCode::Unauthorized.into());
    }
    Ok(())
}

#[derive(Accounts)]
#[instruction(config_account_bump: u8)]
pub struct Initialize<'info> {
    /// singleton account, that searchers must tip
    #[account(
        init,
        seeds = [CONFIG_ACCOUNT_SEED],
        bump = config_account_bump,
        payer = payer,
        space = Config::LEN,
    )]
    pub config: Account<'info, Config>,
    pub initial_tip_claimer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimTips<'info> {
    #[account(
        mut,
        constraint = tip_claimer.key() == config.tip_claimer,
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub tip_claimer: AccountInfo<'info>,
    #[account(mut)]
    pub claimer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ChangeTipClaimer<'info> {
    #[account(
        mut,
        constraint = old_tip_claimer.key() == config.tip_claimer,
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub old_tip_claimer: AccountInfo<'info>,
    pub new_tip_claimer: AccountInfo<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

/// Stores program config metadata.
/// This is the account that searchers must send tips to, for priority block inclusion.
#[account]
#[derive(Default)]
pub struct Config {
    /// account registered by the leader every rotation
    tip_claimer: Pubkey,
}

impl Config {
    // add one byte for header
    pub const LEN: usize = 8 + size_of::<Self>();

    fn calc_tips(total_balance: u64) -> Result<u64> {
        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(Config::LEN);
        Ok(total_balance - min_rent)
    }
}

#[error]
pub enum ErrorCode {
    #[msg("unauthorized instruction call")]
    Unauthorized,
}

/// events

#[event]
pub struct TipsClaimed {
    by: Pubkey,
    to: Pubkey,
    amount: u64,
}

#[event]
pub struct TipClaimerChanged {
    new_tip_claimer: Pubkey,
    old_tip_claimer: Pubkey,
}
