use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::UrthrError;
use crate::state::{Claim, ClaimStatus};

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
}

pub fn handler(ctx: Context<ChallengeClaim>, evidence_hash: [u8; 32]) -> Result<()> {
    let claim = &mut ctx.accounts.claim;
    require!(claim.status == ClaimStatus::Pending, UrthrError::ClaimNotPending);
    let now = Clock::get()?.unix_timestamp;
    require!(now <= claim.challenge_deadline, UrthrError::ChallengeWindowClosed);

    claim.status = ClaimStatus::Challenged;
    claim.challenger = Some(ctx.accounts.challenger.key());
    claim.evidence_hash = evidence_hash;
    Ok(())
}
