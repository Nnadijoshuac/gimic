# On-chain epitaph — inscription proof

The Necromancer can inscribe a ghost's epitaph on-chain as a permanent
gravestone: a Solana transaction carrying a single **SPL Memo** instruction.

**Safety by construction:** the plugin builds *only* a memo transaction
([`src/inscribe.rs`](../src/inscribe.rs)). There is no code path that transfers
SOL or tokens or invokes any other program. The worst an abused signer key can
do through this plugin is spend its own lamports on transaction fees.

## How to inscribe

1. Make a low-balance "gravedigger" signer and fund it with a little SOL:

   ```bash
   cargo run --example gravedigger_keygen      # prints SEED_B58 and PUBKEY
   # fund PUBKEY (devnet: https://faucet.solana.com ; or a few cents of mainnet SOL)
   ```

2. Put the seed in the plugin config and pass `inscribe: true`:

   ```toml
   [[plugins.entries]]
   name = "zeroclaw-necromancer"
   config = { rpc_url = "https://api.mainnet-beta.solana.com", epitaph_signer_key = "<SEED_B58>" }
   ```

   > **summon `<addr>` and inscribe its epitaph**

   The séance output then ends with a `https://solscan.io/tx/<sig>` link to the
   on-chain gravestone.

## Live cluster proof (devnet, `sigVerify: true`)

The public devnet faucet was rate-limited at build time, so the gravedigger
below is unfunded — but the transaction the plugin builds was still submitted to
the **live devnet cluster** for verification via `simulateTransaction`:

- gravedigger pubkey: `9t6YCp3mVX3MeZYGxkhsUqHwZqDTYrWWfnBw6rPC6NDE`
- recent devnet blockhash: `BFiZ66FGfvGxFj24GutWooP5Kj4jPqXoNWBsGNp1hc5Z`
- memo: `⚰ RIP Saint Nullsoul — test epitaph [ZeroClaw Necromancer]`

Cluster response (`sigVerify: true`, `encoding: base58`):

```json
{
  "value": {
    "err": "AccountNotFound",
    "logs": [],
    "preBalances":  [0, 40499599334],
    "postBalances": [0, 40499599334],
    "unitsConsumed": 0
  }
}
```

What this proves:

- **The signature is valid.** With `sigVerify: true`, the cluster verifies the
  ed25519 signature before execution. It did *not* return a signature error — so
  our in-plugin signing over the message bytes is correct.
- **The wire format is valid.** The cluster deserialized our hand-built legacy
  transaction without a sanitize/deserialize error, and resolved account index 1
  to the real **SPL Memo program** (its funded balance shows in `*Balances`).
- **The only failure is funding.** `AccountNotFound` = the 0-balance gravedigger
  can't pay the fee. Fund it and the identical transaction lands.

The transaction builder is also covered by a unit test that reconstructs the
message and verifies the signature offline (`cargo test` →
`memo_tx_is_well_formed_and_correctly_signed`).
