use anchor_lang::prelude::*;

/// Fee basis-point denominator (100% = 10_000 bps).
#[constant]
pub const FEE_DENOMINATOR: u16 = 10_000;

// PDA seed prefixes.
pub const CONFIG_SEED: &[u8] = b"config";
pub const TREASURY_SEED: &[u8] = b"treasury";
pub const PUBLISHER_SEED: &[u8] = b"publisher";
pub const STAKE_VAULT_SEED: &[u8] = b"stake_vault";
pub const CAMPAIGN_SEED: &[u8] = b"campaign";
pub const ESCROW_VAULT_SEED: &[u8] = b"escrow_vault";
pub const CLAIM_SEED: &[u8] = b"claim";
