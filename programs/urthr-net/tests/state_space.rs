use anchor_lang::Space;
use urthr_net::state::{Campaign, Claim, ProtocolConfig, Publisher};

#[test]
fn account_space_is_nonzero_and_bounded() {
    // InitSpace excludes the 8-byte discriminator; all four must be small, fixed accounts.
    assert!(ProtocolConfig::INIT_SPACE > 0 && ProtocolConfig::INIT_SPACE < 512);
    assert!(Publisher::INIT_SPACE > 0 && Publisher::INIT_SPACE < 256);
    assert!(Campaign::INIT_SPACE > 0 && Campaign::INIT_SPACE < 256);
    assert!(Claim::INIT_SPACE > 0 && Claim::INIT_SPACE < 256);
}
