use std::result;

use anchor_lang::{prelude::*, Discriminator};
use solana_program::loader_v4;
use solana_sdk_ids::{
    bpf_loader, bpf_loader_deprecated, bpf_loader_upgradeable, config, native_loader,
    secp256r1_program, sysvar,
};
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

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
    source_revision: std::env!("GIT_SHA"),
    source_release: std::env!("GIT_REF_NAME")
}

declare_id!("T1pyyaTNZsKv2WcRAB8oVnk93mLJw2XzjtVYqCsaHqt");

/// We've decided to hardcode the seeds, effectively meaning the following PDAs owned by this program are singleton.
///
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

struct Fees {
    block_builder_fee_lamports: u64,
    tip_receiver_fee_lamports: u64,
}

impl Fees {
    #[inline(always)]
    fn calculate(
        total_tips: u64,
        block_builder_commission_pct: u64,
    ) -> result::Result<Self, TipPaymentError> {
        let block_builder_fee_lamports = total_tips
            .checked_mul(block_builder_commission_pct)
            .ok_or(TipPaymentError::ArithmeticError)?
            .checked_div(100)
            .ok_or(TipPaymentError::ArithmeticError)?;

        let tip_receiver_fee_lamports = total_tips
            .checked_sub(block_builder_fee_lamports)
            .ok_or(TipPaymentError::ArithmeticError)?;

        Ok(Self {
            block_builder_fee_lamports,
            tip_receiver_fee_lamports,
        })
    }
}

#[program]
pub mod jito_tip_payment {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, _bumps: InitBumps) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.tip_receiver = ctx.accounts.payer.key();
        cfg.block_builder = ctx.accounts.payer.key();
        let mut bumps = InitBumps::default();
        bumps.config = ctx.bumps.config;

        let rent = Rent::get()?;
        bumps.tip_payment_account_0 = TipPaymentAccount::initialize(
            TIP_ACCOUNT_SEED_0,
            ctx.program_id,
            &ctx.accounts.tip_payment_account_0,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            &rent,
        )?;
        bumps.tip_payment_account_1 = TipPaymentAccount::initialize(
            TIP_ACCOUNT_SEED_1,
            ctx.program_id,
            &ctx.accounts.tip_payment_account_1,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            &rent,
        )?;
        bumps.tip_payment_account_2 = TipPaymentAccount::initialize(
            TIP_ACCOUNT_SEED_2,
            ctx.program_id,
            &ctx.accounts.tip_payment_account_2,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            &rent,
        )?;
        bumps.tip_payment_account_3 = TipPaymentAccount::initialize(
            TIP_ACCOUNT_SEED_3,
            ctx.program_id,
            &ctx.accounts.tip_payment_account_3,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            &rent,
        )?;
        bumps.tip_payment_account_4 = TipPaymentAccount::initialize(
            TIP_ACCOUNT_SEED_4,
            ctx.program_id,
            &ctx.accounts.tip_payment_account_4,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            &rent,
        )?;
        bumps.tip_payment_account_5 = TipPaymentAccount::initialize(
            TIP_ACCOUNT_SEED_5,
            ctx.program_id,
            &ctx.accounts.tip_payment_account_5,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            &rent,
        )?;
        bumps.tip_payment_account_6 = TipPaymentAccount::initialize(
            TIP_ACCOUNT_SEED_6,
            ctx.program_id,
            &ctx.accounts.tip_payment_account_6,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            &rent,
        )?;
        bumps.tip_payment_account_7 = TipPaymentAccount::initialize(
            TIP_ACCOUNT_SEED_7,
            ctx.program_id,
            &ctx.accounts.tip_payment_account_7,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            &rent,
        )?;

        cfg.bumps = bumps;
        cfg.block_builder_commission_pct = 0;

        Ok(())
    }

    /// Validator should invoke this instruction before executing any transactions that contain tips.
    /// Validator should also ensure it calls it if there's a fork detected.
    pub fn change_tip_receiver(ctx: Context<ChangeTipReceiver>) -> Result<()> {
        if is_program(&ctx.accounts.new_tip_receiver)
            || is_sysvar(&ctx.accounts.new_tip_receiver)
            || is_config(&ctx.accounts.new_tip_receiver)
        {
            return Err(TipPaymentError::InvalidTipReceiver.into());
        }

        let rent = Rent::get()?;
        let tip_accounts = ctx.accounts.get_tip_accounts();

        handle_payments(
            &rent,
            &tip_accounts,
            &ctx.accounts.old_tip_receiver,
            &ctx.accounts.block_builder,
            ctx.accounts.config.block_builder_commission_pct,
        )?;

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
        if is_program(&ctx.accounts.new_block_builder)
            || is_sysvar(&ctx.accounts.new_block_builder)
            || is_config(&ctx.accounts.new_block_builder)
        {
            return Err(TipPaymentError::InvalidBlockBuilder.into());
        }

        let rent = Rent::get()?;
        let tip_accounts = ctx.accounts.get_tip_accounts();
        handle_payments(
            &rent,
            &tip_accounts,
            &ctx.accounts.tip_receiver,
            &ctx.accounts.old_block_builder,
            // old block builder commission so new block builder can't rug the old one
            ctx.accounts.config.block_builder_commission_pct,
        )?;

        // set new funding account
        ctx.accounts.config.block_builder = ctx.accounts.new_block_builder.key();
        ctx.accounts.config.block_builder_commission_pct = block_builder_commission;
        Ok(())
    }
}

#[inline(always)]
fn is_program(account: &AccountInfo) -> bool {
    *account.owner == bpf_loader::id()
        || *account.owner == bpf_loader_deprecated::id()
        || *account.owner == bpf_loader_upgradeable::id()
        || *account.owner == loader_v4::id()
        || *account.owner == native_loader::id()

        || *account.key == native_loader::id()
        // can remove once feature enable_secp256r1_precompile gets activated
        || *account.key == secp256r1_program::id()

        // note: SIMD-0162 will remove support for this flag: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0162-remove-accounts-executable-flag-checks.md
        || account.executable
}

#[inline(always)]
fn is_sysvar(account: &AccountInfo) -> bool {
    *account.owner == sysvar::id()
}

#[inline(always)]
fn is_config(account: &AccountInfo) -> bool {
    *account.owner == config::id()
}

/// Assumptions:
/// - The transfer_amount are "dangling" lamports and need to be transferred somewhere to have a balanced instruction.
/// - The receiver needs to remain rent exempt
#[inline(always)]
fn transfer_or_credit_tip_pda(
    rent: &Rent,
    receiver: &AccountInfo,
    transfer_amount: u64,
    tip_pda_fallback: &AccountInfo,
) -> Result<u64> {
    let balance_post_transfer = receiver
        .lamports()
        .checked_add(transfer_amount)
        .ok_or(TipPaymentError::ArithmeticError)?;

    // Ensure the transfer amount is greater than 0, the account is rent-exempt after the transfer, and
    // the transfer is not to a program
    let can_transfer = transfer_amount > 0
        && rent.is_exempt(balance_post_transfer, receiver.data_len())
        // programs can't receive lamports until remove_accounts_executable_flag_checks is activated
        && !is_program(receiver);

    if can_transfer {
        **receiver.try_borrow_mut_lamports()? = balance_post_transfer;
        Ok(transfer_amount)
    } else {
        // These lamports can't be left dangling
        let new_tip_pda_balance = tip_pda_fallback
            .lamports()
            .checked_add(transfer_amount)
            .ok_or(TipPaymentError::ArithmeticError)?;
        **tip_pda_fallback.try_borrow_mut_lamports()? = new_tip_pda_balance;
        Ok(0)
    }
}

/// Handles payment of the tips to the block builder and tip receiver
/// Assumptions:
/// - block_builder_commission_percent is a valid number (<= 100)
#[inline(always)]
fn handle_payments(
    rent: &Rent,
    tip_accounts: &[AccountInfo],
    tip_receiver: &AccountInfo,
    block_builder: &AccountInfo,
    block_builder_commission_percent: u64,
) -> Result<()> {
    let total_tips = TipPaymentAccount::drain_accounts(rent, tip_accounts)?;

    let Fees {
        block_builder_fee_lamports,
        tip_receiver_fee_lamports,
    } = Fees::calculate(total_tips, block_builder_commission_percent)?;

    let amount_transferred_to_tip_receiver = if tip_receiver_fee_lamports > 0 {
        let amount_transferred_to_tip_receiver = transfer_or_credit_tip_pda(
            rent,
            tip_receiver,
            tip_receiver_fee_lamports,
            tip_accounts.first().unwrap(),
        )?;
        if amount_transferred_to_tip_receiver == 0 {
            msg!(
                "WARN: did not transfer tip receiver lamports to {:?}",
                tip_receiver.key()
            );
        }
        amount_transferred_to_tip_receiver
    } else {
        0
    };

    let amount_transferred_to_block_builder = if block_builder_fee_lamports > 0 {
        let amount_transferred_to_block_builder = transfer_or_credit_tip_pda(
            rent,
            block_builder,
            block_builder_fee_lamports,
            tip_accounts.first().unwrap(),
        )?;
        if amount_transferred_to_block_builder == 0 {
            msg!(
                "WARN: did not transfer block builder lamports to {:?}",
                block_builder.key()
            );
        }
        amount_transferred_to_block_builder
    } else {
        0
    };

    if amount_transferred_to_tip_receiver > 0 || amount_transferred_to_block_builder > 0 {
        let tip_receiver = if amount_transferred_to_tip_receiver > 0 {
            tip_receiver.key()
        } else {
            Pubkey::default()
        };
        let block_builder = if amount_transferred_to_block_builder > 0 {
            block_builder.key()
        } else {
            Pubkey::default()
        };
        emit!(TipsClaimed {
            tip_receiver,
            tip_receiver_amount: amount_transferred_to_tip_receiver,
            block_builder,
            block_builder_amount: amount_transferred_to_block_builder,
        });
    }
    Ok(())
}

#[error_code]
pub enum TipPaymentError {
    ArithmeticError,
    InvalidFee,
    InvalidTipReceiver,
    InvalidBlockBuilder,
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
    #[account(mut)]
    /// CHECK: Handled in TipPaymentAccount::initialize
    pub tip_payment_account_0: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Handled in TipPaymentAccount::initialize
    pub tip_payment_account_1: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Handled in TipPaymentAccount::initialize
    pub tip_payment_account_2: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Handled in TipPaymentAccount::initialize
    pub tip_payment_account_3: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Handled in TipPaymentAccount::initialize
    pub tip_payment_account_4: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Handled in TipPaymentAccount::initialize
    pub tip_payment_account_5: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Handled in TipPaymentAccount::initialize
    pub tip_payment_account_6: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Handled in TipPaymentAccount::initialize
    pub tip_payment_account_7: UncheckedAccount<'info>,

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
    #[inline(always)]
    fn get_tip_accounts(&self) -> [AccountInfo<'info>; 8] {
        [
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
    #[inline(always)]
    fn get_tip_accounts(&self) -> [AccountInfo<'info>; 8] {
        [
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
    #[inline(always)]
    fn drain_accounts(rent: &Rent, accounts: &[AccountInfo]) -> Result<u64> {
        let mut total_tips: u64 = 0;
        for a in accounts {
            total_tips = total_tips
                .checked_add(Self::drain_account(rent, a)?)
                .ok_or(TipPaymentError::ArithmeticError)?;
        }

        Ok(total_tips)
    }

    #[inline(always)]
    fn drain_account(rent: &Rent, account: &AccountInfo) -> Result<u64> {
        // Tips after rent exemption.
        let tips = account
            .lamports()
            .checked_sub(rent.minimum_balance(account.data_len()))
            .ok_or(TipPaymentError::ArithmeticError)?;

        **account.try_borrow_mut_lamports()? = account
            .lamports()
            .checked_sub(tips)
            .ok_or(TipPaymentError::ArithmeticError)?;

        Ok(tips)
    }

    fn initialize<'info>(
        seeds: &[u8],
        program_id: &Pubkey,
        account_info: &AccountInfo<'info>,
        payer: &AccountInfo<'info>,
        system_program: &AccountInfo<'info>,
        rent: &Rent,
    ) -> Result<u8> {
        let space = TipPaymentAccount::SIZE;

        // Validate PDA
        let (pubkey, bump) = Pubkey::find_program_address(&[seeds], program_id);
        require!(
            &pubkey == account_info.key,
            anchor_lang::error::ErrorCode::ConstraintSeeds
        );

        // CPI to system program to create account
        let current_lamports = account_info.lamports();
        // TODO: Make this error accurate
        // This requirement simplifies the CPIs and checks required
        require!(
            current_lamports == 0,
            anchor_lang::error::ErrorCode::ConstraintSeeds
        );
        let required_lamports = rent.minimum_balance(space);
        let cpi_accounts = anchor_lang::system_program::CreateAccount {
            from: payer.to_account_info(),
            to: account_info.to_account_info(),
        };
        let cpi_context = CpiContext::new(system_program.to_account_info(), cpi_accounts);
        anchor_lang::system_program::create_account(
            cpi_context.with_signer(&[&[seeds, &[bump]]]),
            required_lamports,
            space as u64,
            program_id,
        )?;

        // set the discriminator
        let mut account_data: std::cell::RefMut<'_, &mut [u8]> =
            account_info.try_borrow_mut_data()?;
        account_data[..TipPaymentAccount::DISCRIMINATOR.len()]
            .copy_from_slice(TipPaymentAccount::DISCRIMINATOR);

        Ok(bump)
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
