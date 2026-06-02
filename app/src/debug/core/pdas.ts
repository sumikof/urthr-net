import { getProgramDerivedAddress, getAddressEncoder, address, type Address, type ReadonlyUint8Array } from "@solana/kit";
import { URTHR_NET_PROGRAM_ADDRESS } from "../../generated";

// BPFLoaderUpgradeable — owner of every program's ProgramData account.
const BPF_LOADER_UPGRADEABLE = address("BPFLoaderUpgradeab1e11111111111111111111111");

const enc = getAddressEncoder();
const u64le = (n: bigint) => { const b = new Uint8Array(8); new DataView(b.buffer).setBigUint64(0, n, true); return b; };
const utf8 = (s: string) => new TextEncoder().encode(s);

async function pda(seeds: ReadonlyUint8Array[]): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({ programAddress: URTHR_NET_PROGRAM_ADDRESS, seeds });
  return addr;
}
export const configPda = () => pda([utf8("config")]);
export const treasuryPda = () => pda([utf8("treasury")]);
export const publisherPda = (authority: Address) => pda([utf8("publisher"), enc.encode(authority)]);
export const stakeVaultPda = (authority: Address) => pda([utf8("stake_vault"), enc.encode(authority)]);
export const campaignPda = (advertiser: Address, campaignId: bigint) => pda([utf8("campaign"), enc.encode(advertiser), u64le(campaignId)]);
export const escrowVaultPda = (campaign: Address) => pda([utf8("escrow_vault"), enc.encode(campaign)]);
export const claimPda = (campaign: Address, claimNonce: bigint) => pda([utf8("claim"), enc.encode(campaign), u64le(claimNonce)]);

// ProgramData PDA is [programAddress] under the BPFLoaderUpgradeable, NOT under our program.
export const programDataPda = async (programAddress: Address = URTHR_NET_PROGRAM_ADDRESS): Promise<Address> => {
  const [addr] = await getProgramDerivedAddress({
    programAddress: BPF_LOADER_UPGRADEABLE,
    seeds: [enc.encode(programAddress)],
  });
  return addr;
};
