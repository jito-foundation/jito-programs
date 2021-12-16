use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

const CONFIG_ACCOUNT_SEED: &'static [u8] = b"CONFIG_ACCOUNT";

#[program]
pub mod payment_vault {
    use super::*;

    /// Can only be invoked once due to hardcoded Config account seed.
    pub fn initialize(ctx: Context<Initialize>, _args: InitArgs) -> ProgramResult {
        let cfg = &mut ctx.accounts.config;
        cfg.registered_funding_account = ctx.accounts.initial_funding_account.key();
        Ok(())
    }

    #[access_control(auth_config_account(&ctx.accounts.config, ctx.program_id))]
    /// Validators should call this as the last transaction before they are
    /// rotated out Leadership.
    pub fn claim_funds(ctx: Context<ClaimFunds>) -> ProgramResult {
        let cfg_info = ctx.accounts.config.to_account_info();
        let min_rent = ctx.accounts.rent.minimum_balance(Config::LEN);
        let lamports = cfg_info.lamports() - min_rent;

        if lamports == 0 {
            emit!(FundsClaimed {
                by: ctx.accounts.claimer.key(),
                to: ctx.accounts.registered_funding_account.key(),
                amount: lamports,
            });
            return Ok(());
        }

        // move funds out to registered account
        **cfg_info.try_borrow_mut_lamports()? -= lamports;
        **ctx
            .accounts
            .registered_funding_account
            .try_borrow_mut_lamports()? += lamports;

        emit!(FundsClaimed {
            by: ctx.accounts.claimer.key(),
            to: ctx.accounts.registered_funding_account.key(),
            amount: lamports,
        });

        Ok(())
    }

    #[access_control(auth_config_account(&ctx.accounts.config, ctx.program_id))]
    /// Validator should include this at top of block, at beginning of rotation.
    pub fn register_funding_account(ctx: Context<RegisterFundingAccount>) -> ProgramResult {
        let cfg_info = ctx.accounts.config.to_account_info();
        let min_rent = ctx.accounts.rent.minimum_balance(Config::LEN);
        let lamports = cfg_info.lamports() - min_rent;

        // if Config account has any funds send to previous registered validator
        if lamports != 0 {
            msg!("claiming {} lamports to previous validator", lamports);
            // drain account
            **cfg_info.try_borrow_mut_lamports()? -= lamports;
            // fund old account
            **ctx.accounts.old_funding_account.try_borrow_mut_lamports()? += lamports;
        }

        // set new funding account
        ctx.accounts.config.registered_funding_account = ctx.accounts.new_funding_account.key();

        emit!(FundingAccountRegistered {
            new_funding_account: ctx.accounts.new_funding_account.key(),
            old_funding_account: ctx.accounts.old_funding_account.key(),
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct InitArgs {
    config_account_bump: u8,
}

#[derive(Accounts)]
#[instruction(args: InitArgs)]
pub struct Initialize<'info> {
    /// singleton account, that searchers must tip
    #[account(
        init,
        seeds = [CONFIG_ACCOUNT_SEED],
        bump = args.config_account_bump,
        payer = payer,
        space = Config::LEN,
    )]
    pub config: Account<'info, Config>,
    pub initial_funding_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimFunds<'info> {
    #[account(
        mut,
        constraint = registered_funding_account.key() == config.registered_funding_account,
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub registered_funding_account: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    #[account(mut)]
    pub claimer: Signer<'info>,
}

#[derive(Accounts)]
pub struct RegisterFundingAccount<'info> {
    #[account(
        mut,
        constraint = old_funding_account.key() == config.registered_funding_account,
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub old_funding_account: AccountInfo<'info>,
    pub new_funding_account: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

/// Stores program config metadata.
/// This is the account that searchers
/// must send funds to for priority
/// block inclusion.
#[account]
#[derive(Default)]
pub struct Config {
    /// account registered by the leader every rotation
    registered_funding_account: Pubkey,
}

impl Config {
    // update this if fields are added to the Config struct
    pub const LEN: usize = 8 + 32;
}

#[error]
pub enum ErrorCode {
    #[msg("unauthorized instruction call")]
    Unauthorized,
}

/// events

#[event]
pub struct FundsClaimed {
    by: Pubkey,
    to: Pubkey,
    amount: u64,
}

#[event]
pub struct FundingAccountRegistered {
    new_funding_account: Pubkey,
    old_funding_account: Pubkey,
}
