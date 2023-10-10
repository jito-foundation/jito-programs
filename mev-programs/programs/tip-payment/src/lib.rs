use anchor_lang::prelude::*;
#[cfg(not(feature = "no-entrypoint"))]
use {default_env::default_env, solana_security_txt::security_txt};

use crate::TipPaymentError::ArithmeticError;

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "Jito Tip Payment Program",
    project_url: "https://jito.network/",
    contacts: "email:support@jito.network",
    policy: "https://github.com/jito-foundation/jito-programs",
    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/jito-foundation/jito-programs",
    source_revision: default_env!("GIT_SHA", "GIT_SHA_MISSING"),
    source_release: default_env!("GIT_REF_NAME", "GIT_REF_NAME_MISSING")
}

declare_id!("T1pyyaTNZsKv2WcRAB8oVnk93mLJw2XzjtVYqCsaHqt");

/// We've decided to hardcode the seeds, effectively meaning
/// the following PDAs owned by this program are singleton.
/// This ensures that `initialize` can only be invoked once,
/// otherwise the tx would fail since the accounts would have
/// already been initialized on subsequent calls.
pub const CONFIG_ACCOUNT_SEED: &[u8] = b"CONFIG_ACCOUNT";
pub const TIP_ACCOUNT_SEED_0: &[u8] = b"TIP_ACCOUNT_0";
pub const TIP_ACCOUNT_SEED_1: &[u8] = b"TIP_ACCOUNT_1";
pub const TIP_ACCOUNT_SEED_2: &[u8] = b"TIP_ACCOUNT_2";
pub const TIP_ACCOUNT_SEED_3: &[u8] = b"TIP_ACCOUNT_3";
pub const TIP_ACCOUNT_SEED_4: &[u8] = b"TIP_ACCOUNT_4";
pub const TIP_ACCOUNT_SEED_5: &[u8] = b"TIP_ACCOUNT_5";
pub const TIP_ACCOUNT_SEED_6: &[u8] = b"TIP_ACCOUNT_6";
pub const TIP_ACCOUNT_SEED_7: &[u8] = b"TIP_ACCOUNT_7";

pub const HEADER: usize = 8;

#[program]
pub mod jito_tip_payment {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, _bumps: InitBumps) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.tip_receiver = ctx.accounts.payer.key();
        cfg.block_builder = ctx.accounts.payer.key();

        let bumps = InitBumps {
            config: *ctx.bumps.get("config").unwrap(),
            tip_payment_account_0: *ctx.bumps.get("tip_payment_account_0").unwrap(),
            tip_payment_account_1: *ctx.bumps.get("tip_payment_account_1").unwrap(),
            tip_payment_account_2: *ctx.bumps.get("tip_payment_account_2").unwrap(),
            tip_payment_account_3: *ctx.bumps.get("tip_payment_account_3").unwrap(),
            tip_payment_account_4: *ctx.bumps.get("tip_payment_account_4").unwrap(),
            tip_payment_account_5: *ctx.bumps.get("tip_payment_account_5").unwrap(),
            tip_payment_account_6: *ctx.bumps.get("tip_payment_account_6").unwrap(),
            tip_payment_account_7: *ctx.bumps.get("tip_payment_account_7").unwrap(),
        };
        cfg.bumps = bumps;
        cfg.block_builder_commission_pct = 0;

        Ok(())
    }

    pub fn claim_tips(ctx: Context<ClaimTips>) -> Result<()> {
        let total_tips = TipPaymentAccount::drain_accounts(ctx.accounts.get_tip_accounts())?;

        let block_builder_fee = total_tips
            .checked_mul(ctx.accounts.config.block_builder_commission_pct)
            .ok_or(ArithmeticError)?
            .checked_div(100)
            .ok_or(ArithmeticError)?;

        let tip_receiver_fee = total_tips
            .checked_sub(block_builder_fee)
            .ok_or(ArithmeticError)?;

        if tip_receiver_fee > 0 {
            **ctx.accounts.tip_receiver.try_borrow_mut_lamports()? = ctx
                .accounts
                .tip_receiver
                .lamports()
                .checked_add(tip_receiver_fee)
                .ok_or(ArithmeticError)?;
        }

        if block_builder_fee > 0 {
            **ctx.accounts.block_builder.try_borrow_mut_lamports()? = ctx
                .accounts
                .block_builder
                .lamports()
                .checked_add(block_builder_fee)
                .ok_or(ArithmeticError)?;
        }

        if block_builder_fee > 0 || tip_receiver_fee > 0 {
            emit!(TipsClaimed {
                tip_receiver: ctx.accounts.tip_receiver.key(),
                tip_receiver_amount: tip_receiver_fee,
                block_builder: ctx.accounts.block_builder.key(),
                block_builder_amount: block_builder_fee,
            });
        }

        Ok(())
    }

    /// Validator should invoke this instruction before executing any transactions that contain tips.
    /// Validator should also ensure it calls it if there's a fork detected.
    pub fn change_tip_receiver(ctx: Context<ChangeTipReceiver>) -> Result<()> {
        let total_tips = TipPaymentAccount::drain_accounts(ctx.accounts.get_tip_accounts())?;

        let block_builder_fee = total_tips
            .checked_mul(ctx.accounts.config.block_builder_commission_pct)
            .ok_or(ArithmeticError)?
            .checked_div(100)
            .ok_or(ArithmeticError)?;

        let tip_receiver_fee = total_tips
            .checked_sub(block_builder_fee)
            .ok_or(ArithmeticError)?;

        if tip_receiver_fee > 0 {
            **ctx.accounts.old_tip_receiver.try_borrow_mut_lamports()? = ctx
                .accounts
                .old_tip_receiver
                .lamports()
                .checked_add(tip_receiver_fee)
                .ok_or(ArithmeticError)?;
        }

        if block_builder_fee > 0 {
            **ctx.accounts.block_builder.try_borrow_mut_lamports()? = ctx
                .accounts
                .block_builder
                .lamports()
                .checked_add(block_builder_fee)
                .ok_or(ArithmeticError)?;
        }

        if block_builder_fee > 0 || tip_receiver_fee > 0 {
            emit!(TipsClaimed {
                tip_receiver: ctx.accounts.old_tip_receiver.key(),
                tip_receiver_amount: tip_receiver_fee,
                block_builder: ctx.accounts.block_builder.key(),
                block_builder_amount: block_builder_fee,
            });
        }

        // set new funding account
        ctx.accounts.config.tip_receiver = ctx.accounts.new_tip_receiver.key();
        Ok(())
    }

    /// Changes the block builder. The block builder takes a cut on tips transferred out by
    /// this program. In order for the block builder to be changed, all previous tips must have been
    /// drained.
    pub fn change_block_builder(
        ctx: Context<ChangeBlockBuilder>,
        block_builder_commission: u64,
    ) -> Result<()> {
        require_gte!(100, block_builder_commission, TipPaymentError::InvalidFee);

        let total_tips = TipPaymentAccount::drain_accounts(ctx.accounts.get_tip_accounts())?;

        let block_builder_fee = total_tips
            .checked_mul(ctx.accounts.config.block_builder_commission_pct)
            .ok_or(ArithmeticError)?
            .checked_div(100)
            .ok_or(ArithmeticError)?;

        let tip_receiver_fee = total_tips
            .checked_sub(block_builder_fee)
            .ok_or(ArithmeticError)?;

        if tip_receiver_fee > 0 {
            **ctx.accounts.tip_receiver.try_borrow_mut_lamports()? = ctx
                .accounts
                .tip_receiver
                .lamports()
                .checked_add(tip_receiver_fee)
                .ok_or(ArithmeticError)?;
        }

        if block_builder_fee > 0 {
            **ctx.accounts.old_block_builder.try_borrow_mut_lamports()? = ctx
                .accounts
                .old_block_builder
                .lamports()
                .checked_add(block_builder_fee)
                .ok_or(ArithmeticError)?;
        }

        if block_builder_fee > 0 || tip_receiver_fee > 0 {
            emit!(TipsClaimed {
                tip_receiver: ctx.accounts.tip_receiver.key(),
                tip_receiver_amount: tip_receiver_fee,
                block_builder: ctx.accounts.old_block_builder.key(),
                block_builder_amount: block_builder_fee,
            });
        }

        // set new funding account
        ctx.accounts.config.block_builder = ctx.accounts.new_block_builder.key();
        ctx.accounts.config.block_builder_commission_pct = block_builder_commission;
        Ok(())
    }
}

#[error_code]
pub enum TipPaymentError {
    ArithmeticError,
    InvalidFee,
}

/// Bumps used during initialization
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct InitBumps {
    pub config: u8,
    pub tip_payment_account_0: u8,
    pub tip_payment_account_1: u8,
    pub tip_payment_account_2: u8,
    pub tip_payment_account_3: u8,
    pub tip_payment_account_4: u8,
    pub tip_payment_account_5: u8,
    pub tip_payment_account_6: u8,
    pub tip_payment_account_7: u8,
}

impl InitBumps {
    const SIZE: usize = 9;
}

#[derive(Accounts)]
#[instruction(bumps: InitBumps)]
pub struct Initialize<'info> {
    /// singleton account
    #[account(
        init,
        seeds = [CONFIG_ACCOUNT_SEED],
        bump,
        payer = payer,
        space = Config::SIZE,
        rent_exempt = enforce
    )]
    pub config: Account<'info, Config>,
    #[account(
        init,
        seeds = [TIP_ACCOUNT_SEED_0],
        bump,
        payer = payer,
        space = TipPaymentAccount::SIZE,
        rent_exempt = enforce
    )]
    pub tip_payment_account_0: Account<'info, TipPaymentAccount>,
    #[account(
        init,
        seeds = [TIP_ACCOUNT_SEED_1],
        bump,
        payer = payer,
        space = TipPaymentAccount::SIZE,
        rent_exempt = enforce
    )]
    pub tip_payment_account_1: Account<'info, TipPaymentAccount>,
    #[account(
        init,
        seeds = [TIP_ACCOUNT_SEED_2],
        bump,
        payer = payer,
        space = TipPaymentAccount::SIZE,
        rent_exempt = enforce
    )]
    pub tip_payment_account_2: Account<'info, TipPaymentAccount>,
    #[account(
        init,
        seeds = [TIP_ACCOUNT_SEED_3],
        bump,
        payer = payer,
        space = TipPaymentAccount::SIZE,
        rent_exempt = enforce
    )]
    pub tip_payment_account_3: Account<'info, TipPaymentAccount>,
    #[account(
        init,
        seeds = [TIP_ACCOUNT_SEED_4],
        bump,
        payer = payer,
        space = TipPaymentAccount::SIZE,
        rent_exempt = enforce
    )]
    pub tip_payment_account_4: Account<'info, TipPaymentAccount>,
    #[account(
        init,
        seeds = [TIP_ACCOUNT_SEED_5],
        bump,
        payer = payer,
        space = TipPaymentAccount::SIZE,
        rent_exempt = enforce
    )]
    pub tip_payment_account_5: Account<'info, TipPaymentAccount>,
    #[account(
        init,
        seeds = [TIP_ACCOUNT_SEED_6],
        bump,
        payer = payer,
        space = TipPaymentAccount::SIZE,
        rent_exempt = enforce
    )]
    pub tip_payment_account_6: Account<'info, TipPaymentAccount>,
    #[account(
        init,
        seeds = [TIP_ACCOUNT_SEED_7],
        bump,
        payer = payer,
        space = TipPaymentAccount::SIZE,
        rent_exempt = enforce
    )]
    pub tip_payment_account_7: Account<'info, TipPaymentAccount>,

    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimTips<'info> {
    #[account(
        mut,
        seeds = [CONFIG_ACCOUNT_SEED],
        bump = config.bumps.config,
        rent_exempt = enforce
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_0],
        bump = config.bumps.tip_payment_account_0,
        rent_exempt = enforce
    )]
    pub tip_payment_account_0: Account<'info, TipPaymentAccount>,
    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_1],
        bump = config.bumps.tip_payment_account_1,
        rent_exempt = enforce
    )]
    pub tip_payment_account_1: Account<'info, TipPaymentAccount>,
    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_2],
        bump = config.bumps.tip_payment_account_2,
        rent_exempt = enforce
    )]
    pub tip_payment_account_2: Account<'info, TipPaymentAccount>,
    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_3],
        bump = config.bumps.tip_payment_account_3,
        rent_exempt = enforce
    )]
    pub tip_payment_account_3: Account<'info, TipPaymentAccount>,
    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_4],
        bump = config.bumps.tip_payment_account_4,
        rent_exempt = enforce
    )]
    pub tip_payment_account_4: Account<'info, TipPaymentAccount>,
    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_5],
        bump = config.bumps.tip_payment_account_5,
        rent_exempt = enforce
    )]
    pub tip_payment_account_5: Account<'info, TipPaymentAccount>,
    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_6],
        bump = config.bumps.tip_payment_account_6,
        rent_exempt = enforce
    )]
    pub tip_payment_account_6: Account<'info, TipPaymentAccount>,
    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_7],
        bump = config.bumps.tip_payment_account_7,
        rent_exempt = enforce
    )]
    pub tip_payment_account_7: Account<'info, TipPaymentAccount>,

    /// CHECK: this is the account that is configured to receive tips, which is constantly rotating and
    /// can be an account with a private key to a PDA owned by some other program.
    #[account(
        mut,
        constraint = config.tip_receiver == tip_receiver.key(),
    )]
    pub tip_receiver: AccountInfo<'info>,

    /// CHECK: only the current block builder can get tips
    #[account(
        mut,
        constraint = config.block_builder == block_builder.key(),
    )]
    pub block_builder: AccountInfo<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

impl<'info> ClaimTips<'info> {
    fn get_tip_accounts(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.tip_payment_account_0.to_account_info(),
            self.tip_payment_account_1.to_account_info(),
            self.tip_payment_account_2.to_account_info(),
            self.tip_payment_account_3.to_account_info(),
            self.tip_payment_account_4.to_account_info(),
            self.tip_payment_account_5.to_account_info(),
            self.tip_payment_account_6.to_account_info(),
            self.tip_payment_account_7.to_account_info(),
        ]
    }
}

#[derive(Accounts)]
pub struct ChangeTipReceiver<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    /// CHECK: old_tip_receiver receives the funds in the TipPaymentAccount accounts, so
    /// ensure its the one that's expected
    #[account(mut, constraint = old_tip_receiver.key() == config.tip_receiver)]
    pub old_tip_receiver: AccountInfo<'info>,

    /// CHECK: any new, writable account is allowed as a tip receiver.
    #[account(mut)]
    pub new_tip_receiver: AccountInfo<'info>,

    /// CHECK: old_block_builder receives a % of funds in the TipPaymentAccount accounts, so
    /// ensure it's the account that's expected
    #[account(mut, constraint = block_builder.key() == config.block_builder)]
    pub block_builder: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_0],
        bump = config.bumps.tip_payment_account_0,
        rent_exempt = enforce
    )]
    pub tip_payment_account_0: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_1],
        bump = config.bumps.tip_payment_account_1,
        rent_exempt = enforce
    )]
    pub tip_payment_account_1: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_2],
        bump = config.bumps.tip_payment_account_2,
        rent_exempt = enforce
    )]
    pub tip_payment_account_2: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_3],
        bump = config.bumps.tip_payment_account_3,
        rent_exempt = enforce
    )]
    pub tip_payment_account_3: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_4],
        bump = config.bumps.tip_payment_account_4,
        rent_exempt = enforce
    )]
    pub tip_payment_account_4: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_5],
        bump = config.bumps.tip_payment_account_5,
        rent_exempt = enforce
    )]
    pub tip_payment_account_5: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_6],
        bump = config.bumps.tip_payment_account_6,
        rent_exempt = enforce
    )]
    pub tip_payment_account_6: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_7],
        bump = config.bumps.tip_payment_account_7,
        rent_exempt = enforce
    )]
    pub tip_payment_account_7: Account<'info, TipPaymentAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

impl<'info> ChangeTipReceiver<'info> {
    fn get_tip_accounts(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.tip_payment_account_0.to_account_info(),
            self.tip_payment_account_1.to_account_info(),
            self.tip_payment_account_2.to_account_info(),
            self.tip_payment_account_3.to_account_info(),
            self.tip_payment_account_4.to_account_info(),
            self.tip_payment_account_5.to_account_info(),
            self.tip_payment_account_6.to_account_info(),
            self.tip_payment_account_7.to_account_info(),
        ]
    }
}

#[derive(Accounts)]
pub struct ChangeBlockBuilder<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    /// CHECK: old_tip_receiver receives the funds in the TipPaymentAccount accounts, so
    /// ensure its the one that's expected
    #[account(mut, constraint = tip_receiver.key() == config.tip_receiver)]
    pub tip_receiver: AccountInfo<'info>,

    /// CHECK: old_block_builder receives a % of funds in the TipPaymentAccount accounts, so
    /// ensure it's the account that's expected
    #[account(mut, constraint = old_block_builder.key() == config.block_builder)]
    pub old_block_builder: AccountInfo<'info>,

    /// CHECK: any new, writable account is allowed as block builder
    #[account(mut)]
    pub new_block_builder: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_0],
        bump = config.bumps.tip_payment_account_0,
        rent_exempt = enforce
    )]
    pub tip_payment_account_0: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_1],
        bump = config.bumps.tip_payment_account_1,
        rent_exempt = enforce
    )]
    pub tip_payment_account_1: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_2],
        bump = config.bumps.tip_payment_account_2,
        rent_exempt = enforce
    )]
    pub tip_payment_account_2: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_3],
        bump = config.bumps.tip_payment_account_3,
        rent_exempt = enforce
    )]
    pub tip_payment_account_3: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_4],
        bump = config.bumps.tip_payment_account_4,
        rent_exempt = enforce
    )]
    pub tip_payment_account_4: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_5],
        bump = config.bumps.tip_payment_account_5,
        rent_exempt = enforce
    )]
    pub tip_payment_account_5: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_6],
        bump = config.bumps.tip_payment_account_6,
        rent_exempt = enforce
    )]
    pub tip_payment_account_6: Account<'info, TipPaymentAccount>,

    #[account(
        mut,
        seeds = [TIP_ACCOUNT_SEED_7],
        bump = config.bumps.tip_payment_account_7,
        rent_exempt = enforce
    )]
    pub tip_payment_account_7: Account<'info, TipPaymentAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

impl<'info> ChangeBlockBuilder<'info> {
    fn get_tip_accounts(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.tip_payment_account_0.to_account_info(),
            self.tip_payment_account_1.to_account_info(),
            self.tip_payment_account_2.to_account_info(),
            self.tip_payment_account_3.to_account_info(),
            self.tip_payment_account_4.to_account_info(),
            self.tip_payment_account_5.to_account_info(),
            self.tip_payment_account_6.to_account_info(),
            self.tip_payment_account_7.to_account_info(),
        ]
    }
}

/// Stores program config metadata.
#[account]
#[derive(Default)]
pub struct Config {
    /// The account claiming tips from the mev_payment accounts.
    pub tip_receiver: Pubkey,

    /// Block builder that receives a % of fees
    pub block_builder: Pubkey,
    pub block_builder_commission_pct: u64,

    /// Bumps used to derive PDAs
    pub bumps: InitBumps,
}

impl Config {
    // header, fields, and InitBumps
    pub const SIZE: usize = 8 + 32 + 32 + 8 + InitBumps::SIZE;
}

/// Account that searchers will need to tip for their bundles to be accepted.
/// There will be 8 accounts of this type initialized in order to parallelize bundles.
#[account]
#[derive(Default)]
pub struct TipPaymentAccount {}

impl TipPaymentAccount {
    pub const SIZE: usize = 8;

    /// Drains the tip accounts, leaves enough lamports for rent exemption.
    fn drain_accounts(accounts: Vec<AccountInfo>) -> Result<u64> {
        let mut total_tips: u64 = 0;
        for a in accounts {
            total_tips = total_tips
                .checked_add(Self::drain_account(&a)?)
                .ok_or(ArithmeticError)?;
        }

        Ok(total_tips)
    }

    fn drain_account(account: &AccountInfo) -> Result<u64> {
        // Tips after rent exemption.
        let tips = {
            let rent = Rent::get()?;
            let min_rent = rent.minimum_balance(account.data_len());

            account
                .lamports()
                .checked_sub(min_rent)
                .ok_or(ArithmeticError)
        }?;

        **account.try_borrow_mut_lamports()? = account
            .lamports()
            .checked_sub(tips)
            .ok_or(ArithmeticError)?;

        Ok(tips)
    }
}

/// events
#[event]
pub struct TipsClaimed {
    tip_receiver: Pubkey,
    tip_receiver_amount: u64,
    block_builder: Pubkey,
    block_builder_amount: u64,
}
