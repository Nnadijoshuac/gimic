# 🔍 Solana Wallet Due-Diligence — a ZeroClaw agent

> **DM a Solana address to your agent; get back a counterparty risk report.**
> Age, activity, holdings, spam exposure, program fingerprint → a **LOW /
> CAUTION / ELEVATED** verdict, in seconds, in Telegram.

A [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw) **use case** for the
Superteam × ZeroClaw Solana bounty. **Custody tier T0 — read-only.** The agent
holds no key and can never sign, send, or move funds. The worst a fully
prompt-injected agent can do is give a wrong *opinion*.

📄 **Full write-up:** [`SHOWCASE.md`](SHOWCASE.md) · 🛠 **Setup (an evening):**
[`SETUP.md`](SETUP.md) · 🧾 **Real output:**
[`examples/sample_reports.txt`](examples/sample_reports.txt)

---

## The job

Before you transact with a stranger's wallet — OTC trade, P2P/PIX↔USDC swap,
DAO treasury payout, a new client — you want the 30-second "can I trust this
address?" check. Today that means squinting at Solscan. This agent does it for
you and tells you *why*:

```
🔍 Wallet due-diligence — AwpX…ccUm
Verdict: ⚠️ ELEVATED RISK  (confidence: partial (truncated at 1000))

• Scanned: last 1000 txns (window; full age not shown), 10% failed
• Last activity: 2025-01-31 (549 days ago) — dormant
• Holdings: ◎0.000 SOL, 2 live tokens, 0 empty accounts
• Profile: NFT collector / trader (uses System, Metaplex, Magic Eden, SPL Token)

⚠️ Risk signals:
  - Near-zero SOL — cannot cover its own fees (drained or abandoned?).
  - Dormant 549d — stale; confirm the wallet is still controlled.
✅ Reassuring:
  - Established wallet (>972d of history).

→ Treat as high-risk. Verify identity out-of-band and avoid custody-heavy interactions.
```

## How it's built (correct layering)

- **Tier 1 (the submission): a skill, zero compiled code.**
  [`skills/wallet-diligence/SKILL.md`](skills/wallet-diligence/SKILL.md) drives a
  **stock** ZeroClaw binary via the built-in `http_request` tool. A read-only
  profiler is a skill + the http tool — not a WASM plugin.
- **Channel + auth + memory + cron:**
  [`config/agent.toml`](config/agent.toml) — Telegram (long-poll), peer-group
  authorization, a memory watchlist, and a 6-hour `[cron.watchlist]` monitor.
  The `http_request` allowlist is locked to the RPC host.
- **Tier 3 (optional craft): the same profiler as a sandboxed wasm plugin.**
  This repo's `src/` compiles to a `wasm32-wasip2` tool-plugin component
  (read-only; permissions `["http_client","config_read"]`), with
  [`tools/live-host/`](tools/live-host) — a ~130-line wasmtime host that
  reproduces it live against mainnet without a full ZeroClaw install. Included to
  show craft and to validate the analysis on real data; the **skill is what you
  run**.

## Custody & safety (T0)

No key. No signer. No transaction-building code path anywhere. On-chain text
(token names, memos) is treated as **untrusted data** — an injection attempt
becomes a *risk signal*, not an instruction, and the verdict fails **closed**.
Peer-group auth drops non-owners. Output is shaped small so raw RPC never floods
the model's context. Full threat model + prompt-injection transcript in
[`SHOWCASE.md`](SHOWCASE.md).

## Run the analysis yourself (no agent needed)

```sh
cargo run --example diligence -- 67Pjvywr9R5yadr6hFrnZ4uWqaZSsdxd3SYUsunj5cF9   # live RPC
cargo run --example report                                                       # offline demo
cargo test                                                                       # risk-logic tests

# optional Tier-3 wasm plugin, run through the bundled wasmtime host:
cargo build --release --target wasm32-wasip2
cd tools/live-host && cargo run --release -- <WALLET_ADDRESS>
```

## Repo layout

```
skills/wallet-diligence/   ← the Tier-1 skill (the submission)
config/agent.toml          ← ZeroClaw config: Telegram, RPC allowlist, watchlist cron
SHOWCASE.md                ← the write-up (what/who/features/custody/threat model)
SETUP.md                   ← zero-to-running reproduction guide
examples/                  ← real sample reports + native/offline demos
src/                       ← optional Tier-3 wasm plugin + native reference profiler
tools/live-host/           ← wasmtime host to reproduce the plugin live
wit/                       ← vendored ZeroClaw tool-plugin WIT (for the plugin)
```

## License

MIT
