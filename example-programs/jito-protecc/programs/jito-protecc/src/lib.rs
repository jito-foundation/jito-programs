pub mod sdk;

use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

declare_id!("B7XTCnuyLmbhea4KzhzgPN2JidbeTtTogBCk2M3vDSKf");

/// The pre/post guard instructions should be separate transactions or instructions wrapping the inner contents of a bundle or transaction.
#[program]
pub mod jito_protecc {
    use super::*;

    pub fn close_sol_guarded_state(_ctx: Context<CloseSolGuardedState>) -> Result<()> {
        Ok(())
    }

    pub fn pre_sol_guard(ctx: Context<PreSolGuard>, bump: u8) -> Result<()> {
        let sol_guarded_state = &mut ctx.accounts.sol_guarded_state;
        sol_guarded_state.pre_balance = ctx.accounts.guarded_account.lamports();
        sol_guarded_state.bump = bump;

        Ok(())
    }

    pub fn post_sol_guard(ctx: Context<PostSolGuard>) -> Result<()> {
        if ctx.accounts.guarded_account.lamports() < ctx.accounts.sol_guarded_state.pre_balance {
            Err(Error::AnchorError(AnchorError {
                error_name: "sol guard failure".to_string(),
                error_code_number: 69,
                error_msg: format!(
                    "negative balance change: pre_balance: {}, post_balance: {}",
                    ctx.accounts.sol_guarded_state.pre_balance,
                    ctx.accounts.guarded_account.lamports(),
                ),
                error_origin: None,
                compared_values: None,
            }))
        } else {
            Ok(())
        }
    }

    pub fn close_token_guarded_state(_ctx: Context<CloseTokenGuardedState>) -> Result<()> {
        Ok(())
    }

    pub fn pre_token_guard(ctx: Context<PreTokenGuard>, bump: u8) -> Result<()> {
        let token_guarded_state = &mut ctx.accounts.token_guarded_state;
        token_guarded_state.pre_balance = ctx.accounts.token_account.amount;
        token_guarded_state.bump = bump;

        Ok(())
    }

    pub fn post_token_guard(ctx: Context<PostTokenGuard>) -> Result<()> {
        if ctx.accounts.token_account.amount < ctx.accounts.token_guarded_state.pre_balance {
            Err(Error::AnchorError(AnchorError {
                error_name: "spl_token_state guard failure".to_string(),
                error_code_number: 69,
                error_msg: format!(
                    "negative balance change: pre_balance: {}, post_balance: {}",
                    ctx.accounts.token_guarded_state.pre_balance, ctx.accounts.token_account.amount,
                ),
                error_origin: None,
                compared_values: None,
            }))
        } else {
            Ok(())
        }
    }
}

#[derive(Accounts)]
pub struct CloseTokenGuardedState<'info> {
    #[account(
        mut,
        seeds = [
            GuardedState::SEED,
            token_account.key().as_ref(),
            signer.key().as_ref(),
        ],
        bump = token_guarded_state.bump,
        close = signer
    )]
    pub token_guarded_state: Account<'info, GuardedState>,

    pub token_account: Account<'info, TokenAccount>,

    /// Anyone can crank this instruction.
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseSolGuardedState<'info> {
    /// CHECK: We just care about the account's lamports.
    pub guarded_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            GuardedState::SEED,
            guarded_account.key().as_ref(),
            signer.key().as_ref(),
        ],
        bump = sol_guarded_state.bump,
        close = signer
    )]
    pub sol_guarded_state: Account<'info, GuardedState>,

    /// Anyone can crank this instruction.
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct PreTokenGuard<'info> {
    #[account(
        init_if_needed,
        seeds = [
            GuardedState::SEED,
            token_account.key().as_ref(),
            signer.key().as_ref(),
        ],
        bump,
        space = GuardedState::SIZE,
        payer = signer
    )]
    pub token_guarded_state: Account<'info, GuardedState>,

    pub token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PreSolGuard<'info> {
    /// CHECK: We just care about the account's lamports.
    pub guarded_account: AccountInfo<'info>,

    #[account(
        init_if_needed,
        seeds = [
            GuardedState::SEED,
            guarded_account.key().as_ref(),
            signer.key().as_ref(),
        ],
        bump,
        space = GuardedState::SIZE,
        payer = signer
    )]
    pub sol_guarded_state: Account<'info, GuardedState>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PostTokenGuard<'info> {
    #[account(
        mut,
        seeds = [
            GuardedState::SEED,
            token_account.key().as_ref(),
            signer.key().as_ref(),
        ],
        bump = token_guarded_state.bump,
        close = signer
    )]
    pub token_guarded_state: Account<'info, GuardedState>,

    pub token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct PostSolGuard<'info> {
    /// CHECK: We just care about the account's lamports.
    pub guarded_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [
            GuardedState::SEED,
            guarded_account.key().as_ref(),
            signer.key().as_ref(),
        ],
        bump = sol_guarded_state.bump,
        close = signer
    )]
    pub sol_guarded_state: Account<'info, GuardedState>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

#[account]
#[derive(Default)]
pub struct GuardedState {
    pub pre_balance: u64,
    pub bump: u8,
}

impl GuardedState {
    pub const SEED: &'static [u8] = b"GUARDED_STATE";
    pub const SIZE: usize = 8 + size_of::<Self>();
}
