#!/usr/bin/env node
// Sign manifest.toml with an Ed25519 key, matching zeroclaw's canonicalization
// exactly (crates/zeroclaw-plugins/src/signature.rs):
//
//   * Drop every line whose trimmed form starts with `signature`/`publisher_key`
//     and contains `=`.
//   * Strip a trailing `\r` from each line (Rust's str::lines() semantics).
//   * Remove trailing blank lines.
//   * Join with `\n`. Sign those bytes. Signature = base64url (no pad).
//   * publisher_key = lowercase hex of the raw 32-byte public key.
//
// Usage:
//   node scripts/sign.mjs                 # signs ./manifest.toml in place
//   node scripts/sign.mjs path/to/manifest.toml
//
// The private key is generated on first run and saved to scripts/necromancer.key
// (PKCS#8 PEM). Keep it secret; publish only the printed publisher_key.

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const manifestPath = process.argv[2]
  ? path.resolve(process.argv[2])
  : path.resolve(here, "..", "manifest.toml");
const keyPath = path.resolve(here, "necromancer.key");

// ── key: load or generate ──────────────────────────────────────────────────
let privateKey;
if (fs.existsSync(keyPath)) {
  privateKey = crypto.createPrivateKey(fs.readFileSync(keyPath, "utf8"));
} else {
  ({ privateKey } = crypto.generateKeyPairSync("ed25519"));
  fs.writeFileSync(
    keyPath,
    privateKey.export({ type: "pkcs8", format: "pem" }),
    { mode: 0o600 },
  );
  console.error(`🔑 generated new signing key → ${keyPath} (keep it secret)`);
}
const publicKey = crypto.createPublicKey(privateKey);
const rawPub = Buffer.from(publicKey.export({ format: "jwk" }).x, "base64url");
const publisherKeyHex = rawPub.toString("hex");

// ── canonical bytes ─────────────────────────────────────────────────────────
const raw = fs.readFileSync(manifestPath, "utf8");
const kept = [];
for (let line of raw.split("\n")) {
  if (line.endsWith("\r")) line = line.slice(0, -1); // Rust .lines() strips \r
  const t = line.trim();
  if (t.startsWith("signature") && t.includes("=")) continue;
  if (t.startsWith("publisher_key") && t.includes("=")) continue;
  kept.push(line);
}
while (kept.length && kept[kept.length - 1].trim() === "") kept.pop();
const canonical = Buffer.from(kept.join("\n"), "utf8");

// ── sign ────────────────────────────────────────────────────────────────────
const signature = crypto.sign(null, canonical, privateKey).toString("base64url");

// Rewrite the manifest: canonical body + ONLY the two signing lines. No comment
// or other text may follow — anything that survives canonicalization (i.e. is
// not a stripped `signature`/`publisher_key` line) would change the signed bytes
// and break verification at load time.
const out =
  kept.join("\n") +
  `\n` +
  `signature = "${signature}"\n` +
  `publisher_key = "${publisherKeyHex}"\n`;
fs.writeFileSync(manifestPath, out);

console.log("✅ signed", path.basename(manifestPath));
console.log("   signature    =", signature);
console.log("   publisher_key =", publisherKeyHex);
console.log("\nAdd to your zeroclaw config for strict mode:");
console.log("  [plugins.security]");
console.log('  signature_mode = "strict"');
console.log(`  trusted_publisher_keys = ["${publisherKeyHex}"]`);
