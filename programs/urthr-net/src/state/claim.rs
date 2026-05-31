use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ClaimStatus {
    Pending,
    Challenged,
    Settled,
    Slashed,
}

#[account]
#[derive(InitSpace)]
pub struct Claim {
    pub campaign: Pubkey,
    pub publisher: Pubkey,
    pub event_count: u64,
    pub amount: u64,
    pub merkle_root: [u8; 32],
    pub evidence_hash: [u8; 32],
    pub challenger: Option<Pubkey>,
    pub challenge_deadline: i64,
    pub status: ClaimStatus,
    pub bump: u8,
}
