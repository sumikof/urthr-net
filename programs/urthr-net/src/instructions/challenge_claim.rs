use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Claim, ClaimStatus, Publisher};

#[derive(Accounts)]
pub struct ChallengeClaim<'info> {
    #[account(mut)]
    pub challenger: Signer<'info>,

    #[account(
        mut,
        seeds = [CLAIM_SEED, claim.campaign.as_ref(), &claim.claim_nonce.to_le_bytes()],
        bump = claim.bump,
    )]
    pub claim: Account<'info, Claim>,

    #[account(
        seeds = [PUBLISHER_SEED, publisher.authority.as_ref()],
        bump = publisher.bump,
        constraint = claim.publisher == publisher.key() @ UrthrError::Unauthorized,
    )]
    pub publisher: Account<'info, Publisher>,
}

pub fn handler(ctx: Context<ChallengeClaim>, evidence_hash: [u8; 32]) -> Result<()> {
    let claim = &mut ctx.accounts.claim;
    require!(claim.status == ClaimStatus::Pending, UrthrError::ClaimNotPending);
    let now = Clock::get()?.unix_timestamp;
    require!(now <= claim.challenge_deadline, UrthrError::ChallengeWindowClosed);

    require!(
        ctx.accounts.challenger.key() != ctx.accounts.publisher.authority,
        UrthrError::SelfChallengeNotAllowed
    );

    claim.status = ClaimStatus::Challenged;
    claim.challenger = Some(ctx.accounts.challenger.key());
    claim.evidence_hash = evidence_hash;
    Ok(())
}
