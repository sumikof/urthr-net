import { readFileSync, renameSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { createFromRoot } from "codama";
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor } from "@codama/renderers-js";

const here = dirname(fileURLToPath(import.meta.url));
const idlPath = resolve(here, "../../target/idl/urthr_net.json");
const tmpDir = resolve(here, "../.codama-tmp");
const outDir = resolve(here, "../src/generated");

const anchorIdl = JSON.parse(readFileSync(idlPath, "utf-8"));
const codama = createFromRoot(rootNodeFromAnchor(anchorIdl));

// @codama/renderers-js emits a package layout: <tmp>/package.json + <tmp>/src/generated/*.
// Relocate just the client source to a clean app/src/generated so imports are `../generated`.
rmSync(tmpDir, { recursive: true, force: true });
await codama.accept(renderVisitor(tmpDir));

rmSync(outDir, { recursive: true, force: true });
renameSync(resolve(tmpDir, "src/generated"), outDir);
rmSync(tmpDir, { recursive: true, force: true });

console.log(`Generated client at ${outDir}`);
