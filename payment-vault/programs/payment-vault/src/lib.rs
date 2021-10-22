use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

const CONFIG_ACCOUNT_SEED: &'static [u8] = b"GOKU_CONFIG_ACCOUNT_SEED";
const FEE_ACCOUNT_SEED: &'static [u8] = b"VEGETA_FEE_ACCOUNT_SEED";
const TIP_ACCOUNT_SEED: &'static [u8] = b"RYU_TIP_ACCOUNT_SEED";

#[inline(always)]
const fn fee_bps(bps: u64) -> U64F64 {
    U64F64(((bps as u128) << 64) / 10_000)
}

#[repr(transparent)]
#[derive(Copy, Clone)]
struct U64F64(u128);

/// copy-pasta'd https://github.com/project-serum/serum-dex/blob/0c730d678fd5ec0b07b17465e310a7f2a81b6681/dex/src/fees.rs#L24
#[allow(unused)]
impl U64F64 {
    const ONE: Self = U64F64(1 << 64);

    #[inline(always)]
    const fn add(self, other: U64F64) -> U64F64 {
        U64F64(self.0 + other.0)
    }

    #[inline(always)]
    const fn div(self, other: U64F64) -> u128 {
        self.0 / other.0
    }

    #[inline(always)]
    const fn mul_u64(self, other: u64) -> U64F64 {
        U64F64(self.0 * other as u128)
    }

    #[inline(always)]
    const fn floor(self) -> u64 {
        (self.0 >> 64) as u64
    }

    #[inline(always)]
    const fn frac_part(self) -> u64 {
        self.0 as u64
    }

    #[inline(always)]
    const fn from_int(n: u64) -> Self {
        U64F64((n as u128) << 64)
    }
}

#[program]
pub mod payment_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, args: InitArgs) -> ProgramResult {
        let cfg = &mut ctx.accounts.config;
        cfg.fee_account_pk = ctx.accounts.fee_account.key();
        cfg.tip_account_pk = ctx.accounts.tip_account.key();
        cfg.fee_bps = args.fee_bps;
        cfg.decimals = ctx.accounts.mint.decimals;

        // set authorities of tip_account && fee_account to be the program-owned
        token::set_authority(
            ctx.accounts.into_tip_account_set_authority_ctx(),
            AuthorityType::AccountOwner,
            Some(ctx.accounts.config.key()),
        )?;
        token::set_authority(
            ctx.accounts.into_fee_account_set_authority_ctx(),
            AuthorityType::AccountOwner,
            Some(ctx.accounts.config.key()),
        )?;

        Ok(())
    }

    #[access_control(auth_config_account(&ctx.accounts.config, ctx.program_id))]
    pub fn claim_tips(ctx: Context<ClaimTips>, args: ClaimTipsArgs) -> ProgramResult {
        // xfer fee from tip_account to fee_account
        let cpi_accounts = Transfer {
            from: ctx.accounts.tip_account.to_account_info(),
            to: ctx.accounts.fee_account.to_account_info(),
            authority: ctx.accounts.config.to_account_info(),
        };
        let seeds = [&CONFIG_ACCOUNT_SEED[..], &[args.config_bump]];
        let signer = &[&seeds[..]];
        let cpi_ctx =
            CpiContext::new_with_signer(ctx.accounts.token_program.clone(), cpi_accounts, signer);
        let bps = fee_bps(ctx.accounts.config.fee_bps);
        // calc. exact fee based on amount in tip_account
        let exact_fee: U64F64 = bps.mul_u64(ctx.accounts.tip_account.amount);
        let fee = exact_fee.floor() + ((exact_fee.frac_part() != 0) as u64);
        // xfer to JITO owned fee_account
        token::transfer(cpi_ctx, fee)?;

        // xfer remaining funds from tip_account to validator's acc
        let cpi_accounts = Transfer {
            from: ctx.accounts.tip_account.to_account_info(),
            to: ctx.accounts.validator_token_account.to_account_info(),
            authority: ctx.accounts.config.to_account_info(),
        };
        let cpi_ctx =
            CpiContext::new_with_signer(ctx.accounts.token_program.clone(), cpi_accounts, signer);
        let tip_amount = ctx.accounts.tip_account.amount - fee;
        token::transfer(cpi_ctx, tip_amount)?;

        emit!(FeeTransferred {
            from: ctx.accounts.tip_account.key(),
            to: ctx.accounts.fee_account.key(),
            amount: fee,
        });
        emit!(TipTransferred {
            from: ctx.accounts.tip_account.key(),
            to: ctx.accounts.validator_token_account.key(),
            amount: tip_amount,
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
    fee_bps: u64,
    config_account_bump: u8,
    fee_account_bump: u8,
    tip_account_bump: u8,
}

#[derive(Accounts)]
#[instruction(args: InitArgs)]
pub struct Initialize<'info> {
    /// can only be instantiated once due to hardcoded seed
    #[account(
        init,
        seeds = [CONFIG_ACCOUNT_SEED],
        bump = args.config_account_bump,
        payer = payer,
    )]
    pub config: Account<'info, Config>,
    /// holds fees collected by the protocol
    #[account(
        init,
        seeds = [FEE_ACCOUNT_SEED],
        bump = args.fee_account_bump,
        token::mint = mint,
        token::authority = payer,
        payer = payer,
    )]
    pub fee_account: Account<'info, TokenAccount>,
    /// account that searchers send tips to, in return for pri-inclusion
    #[account(
        init,
        seeds = [TIP_ACCOUNT_SEED],
        bump = args.tip_account_bump,
        token::mint = mint,
        token::authority = payer,
        payer = payer,
    )]
    pub tip_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,

    pub token_program: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub payer: Signer<'info>,
}

impl<'info> Initialize<'info> {
    fn into_fee_account_set_authority_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.fee_account.to_account_info().clone(),
            current_authority: self.payer.to_account_info(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn into_tip_account_set_authority_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.tip_account.to_account_info().clone(),
            current_authority: self.payer.to_account_info(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct ClaimTips<'info> {
    #[account(mut)]
    pub fee_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub tip_account: Account<'info, TokenAccount>,
    pub token_program: AccountInfo<'info>,
    /// validator's token account where tips will be sent to
    #[account(mut)]
    pub validator_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = tip_account.key() == config.tip_account_pk,
        constraint = tip_account.owner == config.key(),
        constraint = fee_account.key() == config.fee_account_pk,
        constraint = fee_account.owner == config.key(),
    )]
    pub config: Account<'info, Config>,
    #[account(
        signer,
        constraint = validator_token_account.owner == *validator.key,
    )]
    pub validator: AccountInfo<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct ClaimTipsArgs {
    config_bump: u8,
}

/// stores program config metadata
#[account]
#[derive(Default)]
pub struct Config {
    // SPL token account for fees collected by the protocol
    fee_account_pk: Pubkey,
    // SPL token account for Leader payments
    tip_account_pk: Pubkey,
    fee_bps: u64,
    decimals: u8,
}

#[error]
pub enum ErrorCode {
    #[msg("unauthorized instruction call")]
    Unauthorized,
}

/// events

#[event]
pub struct FeeTransferred {
    from: Pubkey,
    to: Pubkey,
    amount: u64,
}

#[event]
pub struct TipTransferred {
    from: Pubkey,
    to: Pubkey,
    amount: u64,
}
