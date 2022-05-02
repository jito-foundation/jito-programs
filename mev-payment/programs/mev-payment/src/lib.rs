use std::mem::size_of;

use anchor_lang::prelude::*;

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
const VALIDATOR_META_SEED: &'static [u8] = b"VALIDATOR_META";

pub const HEADER: usize = 8;


#[program]
pub mod mev_payment {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, args: InitArgs) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        // This must be set to some value otherwise the `mut` attribute in a subsequent `set_tip_claimer`
        // call will fail since an uninitialized account cannot have data written to it.
        cfg.tip_claimer = ctx.accounts.payer.key();
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
            url: url,
            validator: ctx.accounts.validator.key(),
        });

        Ok(())
    }

    /// Validators should call this at the end of the slot or prior to rotation.
    /// If this isn't called before the next leader rotation, tips will be transferred
    /// in set_tip_claimer before claimer is updated.
    pub fn claim_tips(ctx: Context<ClaimTips>) -> Result<()> {
        let total_tips = MevPaymentAccount::drain_accounts(ctx.accounts.get_mev_accounts())?;
        let pre_lamports = ctx.accounts.tip_claimer.lamports();
        **ctx.accounts.tip_claimer.try_borrow_mut_lamports()? =
            pre_lamports.checked_add(total_tips).expect(&*format!(
                "claim_tips overflow: [tip_claimer: {}, pre_lamports: {}, total_tips: {}]",
                ctx.accounts.tip_claimer.key(),
                pre_lamports,
                total_tips,
            ));

        emit!(TipsClaimed {
            by: ctx.accounts.claimer.key(),
            to: ctx.accounts.tip_claimer.key(),
            amount: total_tips,
        });

        Ok(())
    }

    /// Validator should invoke this instruction at the top of every block in-case
    /// they're on a fork on the first few blocks.
    pub fn change_tip_receiver(ctx: Context<ChangeTipReceiver>) -> Result<()> {
        let total_tips = MevPaymentAccount::drain_accounts(ctx.accounts.get_mev_accounts())?;

        if total_tips > 0 {
            let pre_lamports = ctx.accounts.old_tip_claimer.lamports();
            **ctx.accounts.old_tip_claimer.try_borrow_mut_lamports()? =
                pre_lamports.checked_add(total_tips).expect(&*format!(
                    "set_tip_claimer overflow: [old_tip_claimer: {}, pre_lamports: {}, total_tips: {}]",
                    ctx.accounts.old_tip_claimer.key(),
                    pre_lamports,
                    total_tips,
                ));
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
    bump,
    payer = payer,
    space = Config::SIZE
    )]
    pub config: Account<'info, Config>,
    #[account(
    init,
    seeds = [MEV_ACCOUNT_SEED_1],
    bump,
    payer = payer,
    space = MevPaymentAccount::SIZE,
    )]
    pub mev_payment_account_1: Account<'info, MevPaymentAccount>,
    #[account(
    init,
    seeds = [MEV_ACCOUNT_SEED_2],
    bump,
    payer = payer,
    space = MevPaymentAccount::SIZE,
    )]
    pub mev_payment_account_2: Account<'info, MevPaymentAccount>,
    #[account(
    init,
    seeds = [MEV_ACCOUNT_SEED_3],
    bump,
    payer = payer,
    space = MevPaymentAccount::SIZE,
    )]
    pub mev_payment_account_3: Account<'info, MevPaymentAccount>,
    #[account(
    init,
    seeds = [MEV_ACCOUNT_SEED_4],
    bump,
    payer = payer,
    space = MevPaymentAccount::SIZE,
    )]
    pub mev_payment_account_4: Account<'info, MevPaymentAccount>,
    #[account(
    init,
    seeds = [MEV_ACCOUNT_SEED_5],
    bump,
    payer = payer,
    space = MevPaymentAccount::SIZE,
    )]
    pub mev_payment_account_5: Account<'info, MevPaymentAccount>,
    #[account(
    init,
    seeds = [MEV_ACCOUNT_SEED_6],
    bump ,
    payer = payer,
    space = MevPaymentAccount::SIZE,
    )]
    pub mev_payment_account_6: Account<'info, MevPaymentAccount>,
    #[account(
    init,
    seeds = [MEV_ACCOUNT_SEED_7],
    bump,
    payer = payer,
    space = MevPaymentAccount::SIZE,
    )]
    pub mev_payment_account_7: Account<'info, MevPaymentAccount>,
    #[account(
    init,
    seeds = [MEV_ACCOUNT_SEED_8],
    bump,
    payer = payer,
    space = MevPaymentAccount::SIZE,
    )]
    pub mev_payment_account_8: Account<'info, MevPaymentAccount>,
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
    /// CHECK: this is the account that is configured to receive tips, which is constantly rotating and
    /// can be an account with a private key to a PDA owned by some other program.
    #[account(mut)]
    pub tip_claimer: AccountInfo<'info>,
    #[account(mut)]
    pub claimer: Signer<'info>,
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
pub struct ChangeTipReceiver<'info> {
    #[account(
    mut,
    constraint = old_tip_claimer.key() == config.tip_claimer,
    )]
    pub config: Account<'info, Config>,

    /// CHECK: constraint check above. old tip claimer gets tokens transferred to them before
    /// new tip claimer.
    #[account(mut)]
    pub old_tip_claimer: AccountInfo<'info>,

    /// CHECK: any new account is allowed as a tip claimer.
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

impl<'info> ChangeTipReceiver<'info> {
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
    bump: u8,
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

impl Config {
    pub const SIZE: usize = HEADER + size_of::<Self>();
}

/// Account that searchers will need to tip for their bundles to be accepted.
/// There will be 8 accounts of this type initialized in order to parallelize bundles.
#[account]
#[derive(Default)]
pub struct MevPaymentAccount {
    bump: u8,
}


impl MevPaymentAccount {
    // add 8 bytes for header
    pub const SIZE: usize = HEADER + size_of::<Self>();

    fn drain_accounts(accs: Vec<AccountInfo>) -> Result<u64> {
        let mut total_tips: u64 = 0;
        for acc in accs {
            total_tips = total_tips.checked_add(Self::drain_account(&acc)?).expect(&*format!(
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
pub struct TipClaimerUpdated {
    new_tip_claimer: Pubkey,
    old_tip_claimer: Pubkey,
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
