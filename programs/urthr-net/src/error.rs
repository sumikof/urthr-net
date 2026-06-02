use anchor_lang::prelude::*;

#[error_code]
pub enum UrthrError {
    #[msg("Signer is not authorized for this action")]
    Unauthorized,
    #[msg("Protocol is paused")]
    ProtocolPaused,
    #[msg("Token mint does not match the protocol payment mint")]
    InvalidMint,
    #[msg("Fee basis points exceed the denominator")]
    InvalidFeeBps,
    #[msg("Stake is below the minimum publisher stake")]
    InsufficientStake,
    #[msg("Requested unstake would drop below locked stake or minimum")]
    StakeLocked,
    #[msg("Campaign budget is insufficient for this claim")]
    InsufficientBudget,
    #[msg("Campaign is not active")]
    CampaignNotActive,
    #[msg("Event count must be greater than zero")]
    InvalidEventCount,
    #[msg("Claim is not in the Pending state")]
    ClaimNotPending,
    #[msg("Claim is not in the Challenged state")]
    ClaimNotChallenged,
    #[msg("Challenge window is still open")]
    ChallengeWindowOpen,
    #[msg("Challenge window has closed")]
    ChallengeWindowClosed,
    #[msg("Campaign still has pending claims")]
    HasPendingClaims,
    #[msg("Price per event must be greater than zero")]
    InvalidPrice,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Unstake amount exceeds the staked balance")]
    UnstakeExceedsBalance,
}
