use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::instructions::settle_claim::settle_payout;
use crate::state::{Campaign, Claim, ClaimStatus, ProtocolConfig, Publisher};
use crate::util::fee_amount;

#[derive(Accounts)]
pub struct ResolveClaim<'info> {
    pub attestor: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = treasury,
        has_one = payment_mint @ UrthrError::InvalidMint,
        constraint = !config.paused @ UrthrError::ProtocolPaused,
        constraint = config.attestor == attestor.key() @ UrthrError::Unauthorized,
    )]
    pub config: Box<Account<'info, ProtocolConfig>>,

    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, campaign.advertiser.as_ref(), &campaign.campaign_id.to_le_bytes()],
        bump = campaign.bump,
        has_one = escrow_vault,
    )]
    pub campaign: Box<Account<'info, Campaign>>,

    #[account(
        mut,
        seeds = [CLAIM_SEED, campaign.key().as_ref(), &claim.claim_nonce.to_le_bytes()],
        bump = claim.bump,
        constraint = claim.publisher == publisher.key() @ UrthrError::Unauthorized,
    )]
    pub claim: Box<Account<'info, Claim>>,

    #[account(mut, token::mint = payment_mint)]
    pub escrow_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub treasury: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [PUBLISHER_SEED, publisher.authority.as_ref()],
        bump = publisher.bump,
        has_one = stake_vault,
    )]
    pub publisher: Box<Account<'info, Publisher>>,

    #[account(mut, token::mint = payment_mint)]
    pub stake_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut, token::mint = payment_mint, token::authority = publisher.authority)]
    pub publisher_token_account: Box<Account<'info, TokenAccount>>,

    pub payment_mint: Box<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ResolveClaim>, fraud: bool) -> Result<()> {
    require!(ctx.accounts.claim.status == ClaimStatus::Challenged, UrthrError::ClaimNotChallenged);
    let amount = ctx.accounts.claim.amount;

    if fraud {
        // Compensation: stake_vault -> escrow_vault, signed by the Publisher PDA.
        let publisher_authority = ctx.accounts.publisher.authority;
        let bump = ctx.accounts.publisher.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            PUBLISHER_SEED,
            publisher_authority.as_ref(),
            &[bump],
        ]];
        let decimals = ctx.accounts.payment_mint.decimals;
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.stake_vault.to_account_info(),
                    mint: ctx.accounts.payment_mint.to_account_info(),
                    to: ctx.accounts.escrow_vault.to_account_info(),
                    authority: ctx.accounts.publisher.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
            decimals,
        )?;

        let campaign = &mut ctx.accounts.campaign;
        // Return locked budget to the advertiser AND add the slashed-stake compensation.
        campaign.locked_budget = campaign.locked_budget
            .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
        campaign.budget_remaining = campaign.budget_remaining
            .checked_add(amount).ok_or(UrthrError::MathOverflow)?  // unlock
            .checked_add(amount).ok_or(UrthrError::MathOverflow)?; // compensation

        let publisher = &mut ctx.accounts.publisher;
        // Invariant: `amount` was locked from staked_amount at submit_claim time,
        // so these checked_subs succeed; they error rather than wrap if that breaks.
        publisher.staked_amount = publisher.staked_amount
            .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
        publisher.locked_amount = publisher.locked_amount
            .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;

        ctx.accounts.claim.status = ClaimStatus::Slashed;
    } else {
        let fee = fee_amount(amount, ctx.accounts.config.protocol_fee_bps)?;
        let payout = amount.checked_sub(fee).ok_or(UrthrError::MathOverflow)?;
        settle_payout(
            &ctx.accounts.token_program,
            &ctx.accounts.escrow_vault,
            &ctx.accounts.treasury,
            &ctx.accounts.publisher_token_account,
            &ctx.accounts.payment_mint,
            &ctx.accounts.campaign,
            payout,
            fee,
        )?;
        let campaign = &mut ctx.accounts.campaign;
        campaign.locked_budget = campaign.locked_budget
            .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
        let publisher = &mut ctx.accounts.publisher;
        publisher.locked_amount = publisher.locked_amount
            .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
        ctx.accounts.claim.status = ClaimStatus::Settled;
    }
    Ok(())
}
