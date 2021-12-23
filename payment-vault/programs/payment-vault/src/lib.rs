use anchor_lang::prelude::*;
use std::mem::size_of;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

const CONFIG_ACCOUNT_SEED: &'static [u8] = b"CONFIG_ACCOUNT";

#[program]
pub mod payment_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, _config_account_bump: u8) -> ProgramResult {
        let cfg = &mut ctx.accounts.config;
        cfg.tip_claimer = ctx.accounts.initial_tip_claimer.key();
        cfg.admin = ctx.accounts.admin.key();

        Ok(())
    }

    #[access_control(check_config_account(&ctx.accounts.config, ctx.program_id))]
    /// Validators should call this at the end of the slot or prior to rotation.
    /// If this isn't called before the next leader rotation, tips will be transferred
    /// in set_tip_claimer before claimer is updated.
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

    #[access_control(check_config_account(&_ctx.accounts.config, _ctx.program_id))]
    pub fn create_mev_payment_account(
        _ctx: Context<CreateMEVPaymentAccount>,
        _mev_payment_bump: u8,
    ) -> ProgramResult {
        Ok(())
    }

    #[access_control(check_config_account(&ctx.accounts.config, ctx.program_id))]
    pub fn set_admin(ctx: Context<SetAdmin>) -> ProgramResult {
        let cfg = &mut ctx.accounts.config;
        cfg.admin = ctx.accounts.new_admin.key();

        emit!(AdminUpdated {
            new_admin: cfg.admin,
            old_admin: ctx.accounts.current_admin.key(),
        });

        Ok(())
    }

    #[access_control(check_config_account(&ctx.accounts.config, ctx.program_id))]
    /// Validator should include this at top of block, at beginning of rotation.
    pub fn set_tip_claimer(ctx: Context<SetTipClaimer>) -> ProgramResult {
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

        emit!(TipClaimerUpdated {
            new_tip_claimer: ctx.accounts.new_tip_claimer.key(),
            old_tip_claimer: ctx.accounts.old_tip_claimer.key(),
        });

        Ok(())
    }
}

fn check_config_account(cfg: &Account<Config>, prog_id: &Pubkey) -> ProgramResult {
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
    /// singleton account
    #[account(
        init,
        seeds = [CONFIG_ACCOUNT_SEED],
        bump = config_account_bump,
        payer = payer,
        space = Config::LEN,
    )]
    pub config: Account<'info, Config>,
    pub admin: AccountInfo<'info>,
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
#[instruction(mev_payment_bump: u8)]
pub struct CreateMEVPaymentAccount<'info> {
    #[account(
        constraint = config.admin == admin.key(),
    )]
    pub config: Account<'info, Config>,
    #[account(
        init,
        seeds = [config.to_account_info().key().as_ref()],
        bump = mev_payment_bump,
        payer = admin,
        space = 8,
    )]
    pub mev_payment_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetAdmin<'info> {
    #[account(
        mut,
        constraint = config.admin == current_admin.key()
    )]
    pub config: Account<'info, Config>,
    pub new_admin: AccountInfo<'info>,
    #[account(mut)]
    pub current_admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetTipClaimer<'info> {
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
#[account]
#[derive(Default)]
pub struct Config {
    /// This account has the authority to create and delete a mev_payment_account.
    admin: Pubkey,
    /// The account registered by the leader every rotation.
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
pub struct AdminUpdated {
    new_admin: Pubkey,
    old_admin: Pubkey,
}

#[event]
pub struct TipsClaimed {
    by: Pubkey,
    to: Pubkey,
    amount: u64,
}

#[event]
pub struct TipClaimerUpdated {
    new_tip_claimer: Pubkey,
    old_tip_claimer: Pubkey,
}
