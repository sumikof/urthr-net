import { TOKEN_PROGRAM_ADDRESS } from "@solana-program/token";
import { SYSTEM_PROGRAM_ADDRESS } from "@solana-program/system";
import { address } from "@solana/kit";

export { TOKEN_PROGRAM_ADDRESS, SYSTEM_PROGRAM_ADDRESS };

/** The Rent sysvar address. Some instructions still take it as an explicit account. */
export const RENT_SYSVAR_ADDRESS = address(
  "SysvarRent111111111111111111111111111111111",
);
