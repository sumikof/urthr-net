use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Campaign, CampaignStatus, Claim, ClaimStatus, ProtocolConfig, Publisher};

#[derive(Accounts)]
pub struct SubmitClaim<'info> {
    #[account(mut)]
    pub publisher_authority: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ UrthrError::ProtocolPaused,
    )]
    pub config: Account<'info, ProtocolConfig>,

    #[account(
        mut,
        seeds = [PUBLISHER_SEED, publisher_authority.key().as_ref()],
        bump = publisher.bump,
        constraint = publisher.authority == publisher_authority.key() @ UrthrError::Unauthorized,
    )]
    pub publisher: Account<'info, Publisher>,

    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, campaign.advertiser.as_ref(), &campaign.campaign_id.to_le_bytes()],
        bump = campaign.bump,
        constraint = campaign.status == CampaignStatus::Active @ UrthrError::CampaignNotActive,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(
        init,
        payer = publisher_authority,
        space = 8 + Claim::INIT_SPACE,
        seeds = [CLAIM_SEED, campaign.key().as_ref(), &campaign.claims_count.to_le_bytes()],
        bump,
    )]
    pub claim: Account<'info, Claim>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<SubmitClaim>, event_count: u64, merkle_root: [u8; 32]) -> Result<()> {
    require!(event_count > 0, UrthrError::InvalidEventCount);

    let campaign = &mut ctx.accounts.campaign;
    let publisher = &mut ctx.accounts.publisher;

    let amount = event_count
        .checked_mul(campaign.price_per_event).ok_or(UrthrError::MathOverflow)?;

    require!(campaign.budget_remaining >= amount, UrthrError::InsufficientBudget);
    let available_stake = publisher.staked_amount
        .checked_sub(publisher.locked_amount).ok_or(UrthrError::MathOverflow)?;
    require!(available_stake >= amount, UrthrError::InsufficientStake);

    // Lock budget (pure accounting on the escrow balance).
    campaign.budget_remaining = campaign.budget_remaining
        .checked_sub(amount).ok_or(UrthrError::MathOverflow)?;
    campaign.locked_budget = campaign.locked_budget
        .checked_add(amount).ok_or(UrthrError::MathOverflow)?;
    // Lock stake.
    publisher.locked_amount = publisher.locked_amount
        .checked_add(amount).ok_or(UrthrError::MathOverflow)?;

    let now = Clock::get()?.unix_timestamp;
    let claim = &mut ctx.accounts.claim;
    claim.campaign = campaign.key();
    claim.publisher = publisher.key();
    claim.claim_nonce = campaign.claims_count; // nonce used in the PDA seed
    claim.event_count = event_count;
    claim.amount = amount;
    claim.merkle_root = merkle_root;
    claim.evidence_hash = [0u8; 32];
    claim.challenger = None;
    let window = i64::try_from(ctx.accounts.config.challenge_window)
        .map_err(|_| UrthrError::MathOverflow)?;
    claim.challenge_deadline = now
        .checked_add(window).ok_or(UrthrError::MathOverflow)?;
    claim.status = ClaimStatus::Pending;
    claim.bump = ctx.bumps.claim;

    campaign.claims_count = campaign.claims_count
        .checked_add(1).ok_or(UrthrError::MathOverflow)?;
    Ok(())
}
