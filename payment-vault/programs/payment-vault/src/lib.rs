#![feature(in_band_lifetimes)]

use anchor_lang::prelude::*;
use std::mem::size_of;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

const CONFIG_ACCOUNT_SEED: &'static [u8] = b"CONFIG_ACCOUNT";

#[program]
pub mod payment_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, _args: InitArgs) -> ProgramResult {
        let cfg = &mut ctx.accounts.config;
        cfg.tip_claimer = ctx.accounts.initial_tip_claimer.key();

        Ok(())
    }

    #[access_control(check_config_account(&ctx.accounts.config, ctx.program_id))]
    /// Validators should call this at the end of the slot or prior to rotation.
    /// If this isn't called before the next leader rotation, tips will be transferred
    /// in set_tip_claimer before claimer is updated.
    pub fn claim_tips(ctx: Context<ClaimTips>) -> ProgramResult {
        let total_tips = MevPaymentAccount::debit_accounts(ctx.accounts.get_mev_accounts())?;
        **ctx.accounts.tip_claimer.try_borrow_mut_lamports()? += total_tips;

        emit!(TipsClaimed {
            by: ctx.accounts.claimer.key(),
            to: ctx.accounts.tip_claimer.key(),
            amount: total_tips,
        });

        Ok(())
    }

    #[access_control(check_config_account(&ctx.accounts.config, ctx.program_id))]
    /// Validator should include this at top of block, at beginning of rotation.
    pub fn set_tip_claimer(ctx: Context<SetTipClaimer>) -> ProgramResult {
        let total_tips = MevPaymentAccount::debit_accounts(ctx.accounts.get_mev_accounts())?;

        if total_tips > 0 {
            msg!(
                "transferring {} lamports to previous tip_claimer",
                total_tips
            );
            **ctx.accounts.old_tip_claimer.try_borrow_mut_lamports()? += total_tips;
            emit!(TipsClaimed {
                by: ctx.accounts.signer.key(),
                to: ctx.accounts.old_tip_claimer.key(),
                amount: total_tips,
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

/// instructions

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct InitArgs {
    pub mev_seed_1: String,
    pub mev_seed_2: String,
    pub mev_seed_3: String,
    pub mev_seed_4: String,
    pub mev_seed_5: String,
    pub mev_seed_6: String,
    pub mev_seed_7: String,
    pub mev_seed_8: String,
    pub mev_bump_1: u8,
    pub mev_bump_2: u8,
    pub mev_bump_3: u8,
    pub mev_bump_4: u8,
    pub mev_bump_5: u8,
    pub mev_bump_6: u8,
    pub mev_bump_7: u8,
    pub mev_bump_8: u8,
    pub config_account_bump: u8,
}

#[derive(Accounts)]
#[instruction(args: InitArgs)]
pub struct Initialize<'info> {
    /// singleton account
    #[account(
        init,
        seeds = [CONFIG_ACCOUNT_SEED],
        bump = args.config_account_bump,
        payer = payer,
    )]
    pub config: Account<'info, Config>,
    #[account(
        init,
        seeds = [args.mev_seed_1.as_bytes()],
        bump = args.mev_bump_1,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_1: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [args.mev_seed_2.as_bytes()],
        bump = args.mev_bump_2,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_2: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [args.mev_seed_3.as_bytes()],
        bump = args.mev_bump_3,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_3: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [args.mev_seed_4.as_bytes()],
        bump = args.mev_bump_4,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_4: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [args.mev_seed_5.as_bytes()],
        bump = args.mev_bump_5,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_5: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [args.mev_seed_6.as_bytes()],
        bump = args.mev_bump_6,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_6: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [args.mev_seed_7.as_bytes()],
        bump = args.mev_bump_7,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_7: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [args.mev_seed_8.as_bytes()],
        bump = args.mev_bump_8,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_8: Account<'info, MevPaymentAccount>,
    pub initial_tip_claimer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimTips<'info> {
    #[account(
        constraint = config.tip_claimer == tip_claimer.key(),
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub mev_payment_account_1: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_2: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_3: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_4: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_5: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_6: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_7: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_8: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub tip_claimer: AccountInfo<'info>,
    #[account(mut)]
    pub claimer: Signer<'info>,
}

impl ClaimTips<'info> {
    fn get_mev_accounts(&self) -> Vec<AccountInfo<'info>> {
        let mut accs = Vec::new();
        accs.push(self.mev_payment_account_1.to_account_info());
        accs.push(self.mev_payment_account_2.to_account_info());
        accs.push(self.mev_payment_account_3.to_account_info());
        accs.push(self.mev_payment_account_4.to_account_info());
        accs.push(self.mev_payment_account_5.to_account_info());
        accs.push(self.mev_payment_account_6.to_account_info());
        accs.push(self.mev_payment_account_7.to_account_info());
        accs.push(self.mev_payment_account_8.to_account_info());

        accs
    }
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
    pub mev_payment_account_1: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_2: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_3: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_4: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_5: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_6: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_7: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub mev_payment_account_8: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl SetTipClaimer<'info> {
    fn get_mev_accounts(&self) -> Vec<AccountInfo<'info>> {
        let mut accs = Vec::new();
        accs.push(self.mev_payment_account_1.to_account_info());
        accs.push(self.mev_payment_account_2.to_account_info());
        accs.push(self.mev_payment_account_3.to_account_info());
        accs.push(self.mev_payment_account_4.to_account_info());
        accs.push(self.mev_payment_account_5.to_account_info());
        accs.push(self.mev_payment_account_6.to_account_info());
        accs.push(self.mev_payment_account_7.to_account_info());
        accs.push(self.mev_payment_account_8.to_account_info());

        accs
    }
}

/// accounts

/// Stores program config metadata.
#[account]
#[derive(Default)]
pub struct Config {
    /// The account claiming tips from the mev_payment accounts.
    tip_claimer: Pubkey,
}

/// Account that searchers will need to tip for their bundles to be accepted.
/// There will be 8 accounts of this type initialized in order to parallelize bundles.
#[account]
#[derive(Default)]
pub struct MevPaymentAccount {}

impl MevPaymentAccount {
    // add one byte for header
    pub const LEN: usize = 8 + size_of::<Self>();

    fn debit_accounts(accs: Vec<AccountInfo>) -> Result<u64> {
        let mut total_tips = 0;
        for acc in accs {
            total_tips += Self::debit(acc)?;
        }

        Ok(total_tips)
    }

    fn debit(acc: AccountInfo) -> Result<u64> {
        let tips = Self::calc_tips(acc.lamports())?;
        if tips > 0 {
            **acc.try_borrow_mut_lamports()? -= tips;
        }
        Ok(tips)
    }

    fn calc_tips(total_balance: u64) -> Result<u64> {
        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(Self::LEN);
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
pub struct TipClaimerUpdated {
    new_tip_claimer: Pubkey,
    old_tip_claimer: Pubkey,
}
