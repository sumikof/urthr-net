// On-chain E2E check: build, sign, send the urthr_net `initialize` instruction
// to the local surfnet using the local CLI keypair, and confirm it lands.
// This validates the deployed program + RPC path end-to-end (the browser wallet
// flow is verified separately/manually).
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { resolve } from "node:path";
import {
  appendTransactionMessageInstruction,
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  getSignatureFromTransaction,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  address,
} from "@solana/kit";

const RPC_URL = process.env.VITE_RPC_URL ?? "http://127.0.0.1:8899";
const WS_URL = process.env.VITE_WS_URL ?? "ws://127.0.0.1:8900";
const PROGRAM_ID = "8CsDf7B1YU9HV136afSbYsY8eV2YeJUk5Sd2CpuLLiSb";
// Anchor discriminator for `initialize` (from target/idl/urthr_net.json), no args/accounts.
const INITIALIZE_DISCRIMINATOR = new Uint8Array([175, 175, 109, 31, 13, 152, 155, 237]);

const keypairPath = resolve(homedir(), ".config/solana/id.json");
const secret = Uint8Array.from(JSON.parse(readFileSync(keypairPath, "utf-8")));
const signer = await createKeyPairSignerFromBytes(secret);

const rpc = createSolanaRpc(RPC_URL);
const rpcSubscriptions = createSolanaRpcSubscriptions(WS_URL);

const ix = {
  programAddress: address(PROGRAM_ID),
  accounts: [],
  data: INITIALIZE_DISCRIMINATOR,
};

const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

const message = pipe(
  createTransactionMessage({ version: 0 }),
  (m) => setTransactionMessageFeePayerSigner(signer, m),
  (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
  (m) => appendTransactionMessageInstruction(ix, m),
);

const signedTx = await signTransactionMessageWithSigners(message);
const signature = getSignatureFromTransaction(signedTx);

const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });
await sendAndConfirm(signedTx, { commitment: "confirmed" });

console.log("OK initialize confirmed");
console.log("signer:", signer.address);
console.log("signature:", signature);
