use std::mem::size_of;

use anchor_lang::prelude::*;

declare_id!("3jarB2NVZc9KouEppPHoAf3v9y77H5R88nyfMLDUzuwm");

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
pub const VALIDATOR_META_SEED: &[u8] = b"VALIDATOR_META";

pub const HEADER: usize = 8;

#[program]
pub mod tip_payment {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, _bumps: InitBumps) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.tip_receiver = ctx.accounts.payer.key();

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

        Ok(())
    }

    pub fn init_validator_meta(
        ctx: Context<InitValidatorMeta>,
        backend_url: String,
        // should be at least the size of backend_url
        _extra_space: u32,
        bump: u8,
    ) -> Result<()> {
        let meta = &mut ctx.accounts.meta;
        meta.backend_url = backend_url;
        meta.bump = bump;

        Ok(())
    }

    pub fn close_validator_meta_account(ctx: Context<CloseValidatorMetaAccount>) -> Result<()> {
        emit!(ValidatorMetaAccountClosed {
            validator: ctx.accounts.validator.key(),
        });

        Ok(())
    }

    pub fn set_backend_url(ctx: Context<SetBackendUrl>, url: String) -> Result<()> {
        let meta = &mut ctx.accounts.meta;
        meta.backend_url = url.clone();

        emit!(ValidatorBackendUrlUpdated {
            url,
            validator: ctx.accounts.validator.key(),
        });

        Ok(())
    }

    pub fn claim_tips(ctx: Context<ClaimTips>) -> Result<()> {
        let total_tips = TipPaymentAccount::drain_accounts(ctx.accounts.get_tip_accounts())?;
        let pre_lamports = ctx.accounts.tip_receiver.lamports();
        **ctx.accounts.tip_receiver.try_borrow_mut_lamports()? =
            pre_lamports.checked_add(total_tips).expect(&*format!(
                "claim_tips overflow: [tip_receiver: {}, pre_lamports: {}, total_tips: {}]",
                ctx.accounts.tip_receiver.key(),
                pre_lamports,
                total_tips,
            ));

        emit!(TipsClaimed {
            by: ctx.accounts.signer.key(),
            to: ctx.accounts.tip_receiver.key(),
            amount: total_tips,
        });

        Ok(())
    }

    /// Validator should invoke this instruction before executing any transactions that contain tips.
    /// Validator should also ensure it calls it if there's a fork detected.
    pub fn change_tip_receiver(ctx: Context<ChangeTipReceiver>) -> Result<()> {
        let total_tips = TipPaymentAccount::drain_accounts(ctx.accounts.get_tip_accounts())?;

        if total_tips > 0 {
            let pre_lamports = ctx.accounts.old_tip_receiver.lamports();
            **ctx.accounts.old_tip_receiver.try_borrow_mut_lamports()? =
                pre_lamports.checked_add(total_tips).expect(&*format!(
                    "change_tip_receiver overflow: [old_tip_receiver: {}, pre_lamports: {}, total_tips: {}]",
                    ctx.accounts.old_tip_receiver.key(),
                    pre_lamports,
                    total_tips,
                ));
            emit!(TipsClaimed {
                by: ctx.accounts.signer.key(),
                to: ctx.accounts.old_tip_receiver.key(),
                amount: total_tips,
            });
        }

        // set new funding account
        ctx.accounts.config.tip_receiver = ctx.accounts.new_tip_receiver.key();

        emit!(TipReceiverUpdate {
            new_tip_receiver: ctx.accounts.new_tip_receiver.key(),
            old_tip_receiver: ctx.accounts.old_tip_receiver.key(),
        });

        Ok(())
    }
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
    #[account(mut,
        constraint = config.tip_receiver == tip_receiver.key(),
    )]
    pub tip_receiver: AccountInfo<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_backend_url: String, extra_space: u32, bump: u8)]
pub struct InitValidatorMeta<'info> {
    #[account(
        init,
        seeds = [VALIDATOR_META_SEED, validator.key().as_ref()],
        bump,
        payer = validator,
        space = ValidatorMeta::SIZE + extra_space as usize,
    )]
    pub meta: Account<'info, ValidatorMeta>,
    #[account(mut)]
    pub validator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseValidatorMetaAccount<'info> {
    #[account(
        mut,
        close = validator,
        seeds = [VALIDATOR_META_SEED, validator.key().as_ref()],
        bump = meta.bump,
    )]
    pub meta: Account<'info, ValidatorMeta>,
    #[account(mut)]
    pub validator: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetBackendUrl<'info> {
    #[account(
        mut,
        seeds = [VALIDATOR_META_SEED, validator.key().as_ref()],
        bump = meta.bump,
    )]
    pub meta: Account<'info, ValidatorMeta>,
    pub validator: Signer<'info>,
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
    #[account(
        mut,
        constraint = old_tip_receiver.key() == config.tip_receiver,
    )]
    pub config: Account<'info, Config>,

    /// CHECK: constraint check above. old tip receiver gets tokens transferred to them before
    /// new tip receiver.
    #[account(mut)]
    pub old_tip_receiver: AccountInfo<'info>,

    /// CHECK: any new account is allowed as a tip receiver.
    pub new_tip_receiver: AccountInfo<'info>,
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

/// accounts

/// Stores program config metadata.
#[account]
#[derive(Default)]
pub struct Config {
    /// The account claiming tips from the mev_payment accounts.
    pub tip_receiver: Pubkey,

    /// Bumps used to derive PDAs
    pub bumps: InitBumps,
}

impl Config {
    pub const SIZE: usize = 8 + 32 + 9; // 8 for header, 32 for pubkey, 9 for bumps
}

/// Account that searchers will need to tip for their bundles to be accepted.
/// There will be 8 accounts of this type initialized in order to parallelize bundles.
#[account]
#[derive(Default)]
pub struct TipPaymentAccount {}

impl TipPaymentAccount {
    pub const SIZE: usize = 8;

    fn drain_accounts(accs: Vec<AccountInfo>) -> Result<u64> {
        let mut total_tips: u64 = 0;
        for acc in accs {
            total_tips = total_tips
                .checked_add(Self::drain_account(&acc)?)
                .expect(&*format!(
                    "debit_accounts overflow: [account: {}, amount: {}]",
                    acc.key(),
                    acc.lamports(),
                ));
        }

        Ok(total_tips)
    }

    fn drain_account(acc: &AccountInfo) -> Result<u64> {
        let tips = Self::calc_tips(acc.lamports())?;
        if tips > 0 {
            let pre_lamports = acc.lamports();
            **acc.try_borrow_mut_lamports()? = pre_lamports.checked_sub(tips).expect(&*format!(
                "debit account overflow: [account: {}, pre_lamports: {}, tips: {}]",
                acc.key(),
                pre_lamports,
                tips,
            ));
        }
        Ok(tips)
    }

    fn calc_tips(total_balance: u64) -> Result<u64> {
        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(Self::SIZE);

        Ok(total_balance.checked_sub(min_rent).expect(&*format!(
            "calc_tips overflow: [total_balance: {}, min_rent: {}]",
            total_balance, min_rent,
        )))
    }
}

/// Searchers will need to connect to `backend_url` to stream txs and submit bundles.
/// This may be JITO's hosted backend or custom infrastructure.
#[account]
#[derive(Default)]
pub struct ValidatorMeta {
    pub backend_url: String,
    pub bump: u8,
}

impl ValidatorMeta {
    // subtract size_of::<String>() to allow user to supply the program with extra space needed
    pub const SIZE: usize = HEADER + size_of::<Self>() - size_of::<String>();
}

/// events
#[event]
pub struct TipsClaimed {
    by: Pubkey,
    to: Pubkey,
    amount: u64,
}

#[event]
pub struct TipReceiverUpdate {
    new_tip_receiver: Pubkey,
    old_tip_receiver: Pubkey,
}

#[event]
pub struct ValidatorBackendUrlUpdated {
    url: String,
    validator: Pubkey,
}

#[event]
pub struct ValidatorMetaAccountClosed {
    validator: Pubkey,
}
