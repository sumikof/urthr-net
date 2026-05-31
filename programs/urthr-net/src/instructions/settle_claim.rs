use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Campaign, Claim, ClaimStatus, ProtocolConfig, Publisher};
use crate::util::fee_amount;

#[derive(Accounts)]
pub struct SettleClaim<'info> {
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = treasury,
        has_one = payment_mint @ UrthrError::InvalidMint,
        constraint = !config.paused @ UrthrError::ProtocolPaused,
    )]
    pub config: Account<'info, ProtocolConfig>,

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

    #[account(mut)]
    pub escrow_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub treasury: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [PUBLISHER_SEED, publisher.authority.as_ref()],
        bump = publisher.bump,
    )]
    pub publisher: Box<Account<'info, Publisher>>,

    #[account(mut, token::mint = payment_mint, token::authority = publisher.authority)]
    pub publisher_token_account: Box<Account<'info, TokenAccount>>,

    pub payment_mint: Box<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<SettleClaim>) -> Result<()> {
    let claim = &ctx.accounts.claim;
    require!(claim.status == ClaimStatus::Pending, UrthrError::ClaimNotPending);
    let now = Clock::get()?.unix_timestamp;
    require!(now > claim.challenge_deadline, UrthrError::ChallengeWindowOpen);

    let amount = claim.amount;
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

    // Accounting: consume locked budget, release locked stake.
    // budget_remaining is NOT restored — the tokens left the escrow vault above.
    let campaign = &mut ctx.accounts.campaign;
    campaign.locked_budget = campaign.locked_budget
        .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
    let publisher = &mut ctx.accounts.publisher;
    publisher.locked_amount = publisher.locked_amount
        .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
    ctx.accounts.claim.status = ClaimStatus::Settled;
    Ok(())
}

/// Shared escrow->publisher/treasury transfer, signed by the Campaign PDA.
/// Reused by resolve_claim (settle branch) in Task 11. Keep this signature stable.
#[allow(clippy::too_many_arguments)]
pub fn settle_payout<'info>(
    token_program: &Program<'info, Token>,
    escrow_vault: &Account<'info, TokenAccount>,
    treasury: &Account<'info, TokenAccount>,
    publisher_token_account: &Account<'info, TokenAccount>,
    payment_mint: &Account<'info, Mint>,
    campaign: &Account<'info, Campaign>,
    payout: u64,
    fee: u64,
) -> Result<()> {
    let advertiser = campaign.advertiser;
    let campaign_id = campaign.campaign_id.to_le_bytes();
    let signer_seeds: &[&[&[u8]]] = &[&[
        CAMPAIGN_SEED,
        advertiser.as_ref(),
        &campaign_id,
        &[campaign.bump],
    ]];
    let decimals = payment_mint.decimals;

    transfer_checked(
        CpiContext::new_with_signer(
            token_program.key(),
            TransferChecked {
                from: escrow_vault.to_account_info(),
                mint: payment_mint.to_account_info(),
                to: publisher_token_account.to_account_info(),
                authority: campaign.to_account_info(),
            },
            signer_seeds,
        ),
        payout,
        decimals,
    )?;

    if fee > 0 {
        transfer_checked(
            CpiContext::new_with_signer(
                token_program.key(),
                TransferChecked {
                    from: escrow_vault.to_account_info(),
                    mint: payment_mint.to_account_info(),
                    to: treasury.to_account_info(),
                    authority: campaign.to_account_info(),
                },
                signer_seeds,
            ),
            fee,
            decimals,
        )?;
    }
    Ok(())
}
