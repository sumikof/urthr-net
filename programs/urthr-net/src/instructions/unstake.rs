use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{ProtocolConfig, Publisher};

#[derive(Accounts)]
pub struct Unstake<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = payment_mint @ UrthrError::InvalidMint,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [PUBLISHER_SEED, authority.key().as_ref()],
        bump = publisher.bump,
        has_one = authority @ UrthrError::Unauthorized,
        has_one = stake_vault,
    )]
    pub publisher: Account<'info, Publisher>,

    #[account(mut, token::mint = payment_mint)]
    pub stake_vault: Account<'info, TokenAccount>,

    #[account(mut, token::mint = payment_mint, token::authority = authority)]
    pub authority_token_account: Account<'info, TokenAccount>,

    pub payment_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Unstake>, amount: u64) -> Result<()> {
    let publisher = &ctx.accounts.publisher;
    let remaining = publisher.staked_amount
        .checked_sub(amount).ok_or(UrthrError::UnstakeExceedsBalance)?;
    require!(remaining >= publisher.locked_amount, UrthrError::StakeLocked);
    require!(
        remaining == 0 || remaining >= ctx.accounts.config.min_publisher_stake,
        UrthrError::StakeLocked
    );

    let authority_key = ctx.accounts.authority.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        PUBLISHER_SEED,
        authority_key.as_ref(),
        &[publisher.bump],
    ]];

    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.stake_vault.to_account_info(),
                mint: ctx.accounts.payment_mint.to_account_info(),
                to: ctx.accounts.authority_token_account.to_account_info(),
                authority: ctx.accounts.publisher.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
        ctx.accounts.payment_mint.decimals,
    )?;

    let publisher = &mut ctx.accounts.publisher;
    publisher.staked_amount = remaining;
    Ok(())
}
