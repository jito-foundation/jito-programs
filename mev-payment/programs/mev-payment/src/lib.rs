use anchor_lang::prelude::*;
use std::mem::size_of;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

/// We've decided to hardcode the seeds, effectively meaning
/// the following PDAs owned by this program are singleton.
/// This ensures that `initialize` can only be invoked once,
/// otherwise the tx would fail since the accounts would have
/// already been initialized on subsequent calls.
const CONFIG_ACCOUNT_SEED: &'static [u8] = b"CONFIG_ACCOUNT";
const MEV_ACCOUNT_SEED_1: &'static [u8] = b"MEV_ACCOUNT_1";
const MEV_ACCOUNT_SEED_2: &'static [u8] = b"MEV_ACCOUNT_2";
const MEV_ACCOUNT_SEED_3: &'static [u8] = b"MEV_ACCOUNT_3";
const MEV_ACCOUNT_SEED_4: &'static [u8] = b"MEV_ACCOUNT_4";
const MEV_ACCOUNT_SEED_5: &'static [u8] = b"MEV_ACCOUNT_5";
const MEV_ACCOUNT_SEED_6: &'static [u8] = b"MEV_ACCOUNT_6";
const MEV_ACCOUNT_SEED_7: &'static [u8] = b"MEV_ACCOUNT_7";
const MEV_ACCOUNT_SEED_8: &'static [u8] = b"MEV_ACCOUNT_8";

#[program]
pub mod mev_payment {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, args: InitArgs) -> ProgramResult {
        let cfg = &mut ctx.accounts.config;
        // This must be set to some value otherwise the `mut` attribute in a subsequent `set_tip_claimer`
        // call will fail since an uninitialized account cannot have data written to it.
        cfg.tip_claimer = ctx.accounts.initial_tip_claimer.key();
        cfg.mev_bump_1 = args.mev_bump_1;
        cfg.mev_bump_2 = args.mev_bump_2;
        cfg.mev_bump_3 = args.mev_bump_3;
        cfg.mev_bump_4 = args.mev_bump_4;
        cfg.mev_bump_5 = args.mev_bump_5;
        cfg.mev_bump_6 = args.mev_bump_6;
        cfg.mev_bump_7 = args.mev_bump_7;
        cfg.mev_bump_8 = args.mev_bump_8;

        Ok(())
    }

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

    /// Validator should invoke this instruction at the top of every block in-case
    /// they're on a fork on the first few blocks.
    pub fn set_tip_claimer(ctx: Context<SetTipClaimer>) -> ProgramResult {
        let total_tips = MevPaymentAccount::debit_accounts(ctx.accounts.get_mev_accounts())?;

        if total_tips > 0 {
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

/// instructions

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct InitArgs {
    pub config_account_bump: u8,
    pub mev_bump_1: u8,
    pub mev_bump_2: u8,
    pub mev_bump_3: u8,
    pub mev_bump_4: u8,
    pub mev_bump_5: u8,
    pub mev_bump_6: u8,
    pub mev_bump_7: u8,
    pub mev_bump_8: u8,
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
        seeds = [MEV_ACCOUNT_SEED_1],
        bump = args.mev_bump_1,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_1: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [MEV_ACCOUNT_SEED_2],
        bump = args.mev_bump_2,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_2: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [MEV_ACCOUNT_SEED_3],
        bump = args.mev_bump_3,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_3: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [MEV_ACCOUNT_SEED_4],
        bump = args.mev_bump_4,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_4: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [MEV_ACCOUNT_SEED_5],
        bump = args.mev_bump_5,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_5: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [MEV_ACCOUNT_SEED_6],
        bump = args.mev_bump_6,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_6: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [MEV_ACCOUNT_SEED_7],
        bump = args.mev_bump_7,
        payer = payer,
        space = MevPaymentAccount::LEN,
    )]
    pub mev_payment_account_7: Account<'info, MevPaymentAccount>,
    #[account(
        init,
        seeds = [MEV_ACCOUNT_SEED_8],
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
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_1],
        bump = config.mev_bump_1
    )]
    pub mev_payment_account_1: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_2],
        bump = config.mev_bump_2
    )]
    pub mev_payment_account_2: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_3],
        bump = config.mev_bump_3
    )]
    pub mev_payment_account_3: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_4],
        bump = config.mev_bump_4
    )]
    pub mev_payment_account_4: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_5],
        bump = config.mev_bump_5
    )]
    pub mev_payment_account_5: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_6],
        bump = config.mev_bump_6
    )]
    pub mev_payment_account_6: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_7],
        bump = config.mev_bump_7
    )]
    pub mev_payment_account_7: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_8],
        bump = config.mev_bump_8
    )]
    pub mev_payment_account_8: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub tip_claimer: AccountInfo<'info>,
    #[account(mut)]
    pub claimer: Signer<'info>,
}

impl<'info> ClaimTips<'info> {
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
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_1],
        bump = config.mev_bump_1
    )]
    pub mev_payment_account_1: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_2],
        bump = config.mev_bump_2
    )]
    pub mev_payment_account_2: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_3],
        bump = config.mev_bump_3
    )]
    pub mev_payment_account_3: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_4],
        bump = config.mev_bump_4
    )]
    pub mev_payment_account_4: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_5],
        bump = config.mev_bump_5
    )]
    pub mev_payment_account_5: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_6],
        bump = config.mev_bump_6
    )]
    pub mev_payment_account_6: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_7],
        bump = config.mev_bump_7
    )]
    pub mev_payment_account_7: Account<'info, MevPaymentAccount>,
    #[account(
        mut,
        seeds = [MEV_ACCOUNT_SEED_8],
        bump = config.mev_bump_8
    )]
    pub mev_payment_account_8: Account<'info, MevPaymentAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl<'info> SetTipClaimer<'info> {
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
    /// Bumps used to derive MEV account PDAs.
    mev_bump_1: u8,
    mev_bump_2: u8,
    mev_bump_3: u8,
    mev_bump_4: u8,
    mev_bump_5: u8,
    mev_bump_6: u8,
    mev_bump_7: u8,
    mev_bump_8: u8,
}

/// Account that searchers will need to tip for their bundles to be accepted.
/// There will be 8 accounts of this type initialized in order to parallelize bundles.
#[account]
#[derive(Default)]
pub struct MevPaymentAccount {}

impl MevPaymentAccount {
    // add one byte for header
    pub const LEN: usize = 8 + size_of::<Self>();

    fn debit_accounts(accs: Vec<AccountInfo>) -> Result<u64, ProgramError> {
        let mut total_tips = 0;
        for acc in accs {
            total_tips += Self::debit(acc)?;
        }

        Ok(total_tips)
    }

    fn debit(acc: AccountInfo) -> Result<u64, ProgramError> {
        let tips = Self::calc_tips(acc.lamports())?;
        if tips > 0 {
            **acc.try_borrow_mut_lamports()? -= tips;
        }
        Ok(tips)
    }

    fn calc_tips(total_balance: u64) -> Result<u64, ProgramError> {
        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(Self::LEN);
        Ok(total_balance - min_rent)
    }
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
