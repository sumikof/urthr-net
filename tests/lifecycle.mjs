// UrthrNet integration smoke test (Surfpool / surfnet).
//
// WHAT THIS IS
// ------------
// A zero-dependency Node script that, when a local surfnet is running, verifies
// the RPC is healthy and reports whether the urthr_net program is deployed at the
// declared program id. If no surfnet is reachable it prints a skip notice and
// exits 0 (so it never fails CI in environments without a validator).
//
// WHY IT IS A SMOKE TEST (and not a full E2E here)
// ------------------------------------------------
// The AUTHORITATIVE correctness gate for the protocol is the Rust LiteSVM suite
// (`cargo test -p urthr-net`, 27 tests). LiteSVM runs the *real* SPL Token program,
// so every instruction — including the token-moving settle and slash paths — is
// exercised end to end there:
//   fund -> stake -> submit_claim -> (window) -> settle_claim         (happy path)
//   submit_claim -> challenge_claim -> resolve_claim(fraud=true)      (slash path)
//   submit -> settle -> close_campaign                               (refund path)
//
// A full on-chain E2E against a surfnet additionally requires deploying the program
// at its declared id (`3CmDt8Rps32EUpnDp9aDWq9GuwZ6WpTp8YxxMLFDRPoR`). The repo's
// current `target/deploy` keypair does not match that id, so a real deploy needs the
// matching program keypair (or `anchor keys sync` to rotate the id) — a deploy-time
// concern out of scope for this protocol-core milestone. When that keypair is
// available (e.g. in CI), extend this script with the lifecycle flows above using a
// generated `@solana/kit` client.
//
// USAGE
// -----
//   NO_DNA=1 surfpool start            # in a separate shell
//   node tests/lifecycle.mjs           # or: pnpm test:integration
//   RPC_URL=http://127.0.0.1:8899 node tests/lifecycle.mjs

const RPC_URL = process.env.RPC_URL ?? "http://127.0.0.1:8899";
const PROGRAM_ID = "3CmDt8Rps32EUpnDp9aDWq9GuwZ6WpTp8YxxMLFDRPoR";

async function rpc(method, params = []) {
  const res = await fetch(RPC_URL, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const json = await res.json();
  if (json.error) throw new Error(`RPC ${method}: ${json.error.message}`);
  return json.result;
}

async function main() {
  let health;
  try {
    health = await rpc("getHealth");
  } catch (err) {
    console.log(`⏭  SKIP: no surfnet reachable at ${RPC_URL} (${err.message}).`);
    console.log("   Start one with `NO_DNA=1 surfpool start`, then re-run.");
    console.log("   Protocol correctness is gated by the Rust LiteSVM suite:");
    console.log("     cargo test -p urthr-net   (27 tests)");
    process.exit(0);
  }

  if (health !== "ok") {
    console.error(`❌ surfnet health is '${health}', expected 'ok'`);
    process.exit(1);
  }
  console.log(`✓ surfnet healthy at ${RPC_URL}`);

  const acct = await rpc("getAccountInfo", [PROGRAM_ID, { encoding: "base64" }]);
  if (!acct || !acct.value) {
    console.log(`⏭  surfnet is up but program ${PROGRAM_ID} is not deployed.`);
    console.log("   Deploy it (matching the declared id), then re-run for the program check.");
    process.exit(0);
  }
  if (!acct.value.executable) {
    console.error(`❌ account ${PROGRAM_ID} exists but is not executable (not a program).`);
    process.exit(1);
  }
  console.log(`✓ program ${PROGRAM_ID} is deployed and executable`);
  console.log("✅ surfnet integration smoke test passed.");
  process.exit(0);
}

main().catch((err) => {
  console.error(`❌ unexpected error: ${err.stack ?? err}`);
  process.exit(1);
});
