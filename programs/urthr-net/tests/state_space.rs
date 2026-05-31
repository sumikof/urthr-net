use anchor_lang::Space;
use urthr_net::state::{Campaign, Claim, ProtocolConfig, Publisher};

#[test]
fn account_space_matches_exact_layout() {
    // InitSpace excludes the 8-byte discriminator. Exact sizes guard against
    // accidental field additions/removals/reorders changing the on-chain layout.
    assert_eq!(ProtocolConfig::INIT_SPACE, 148); // 4*32 + 2 + 8 + 8 + 1 + 1
    assert_eq!(Publisher::INIT_SPACE, 113); // 2*32 + 8 + 8 + 32 + 1
    assert_eq!(Campaign::INIT_SPACE, 106); // 2*32 + 5*8 + 1 + 1
    assert_eq!(Claim::INIT_SPACE, 195); // 2*32 + 8 + 8 + 8 + 2*32 + 33 + 8 + 1 + 1
}
