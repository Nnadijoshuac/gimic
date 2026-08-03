# wallet-diligence skill

A ZeroClaw skill that turns a stock agent into a **Solana wallet
due-diligence** desk. DM it an address in Telegram (or any channel); it reads
the wallet's on-chain history via the built-in `http_request` tool and replies
with a compact risk report — profile, risk signals, and a LOW / CAUTION /
ELEVATED verdict.

- **Custody tier T0 (read-only).** No key, no signing, no transactions.
- **Zero compiled code.** Runs on the stock release binary using `http_request`.
- **Context-safe.** Output is shaped to a short report, never raw RPC.

## Install (stock binary)

```sh
zeroclaw skills bundle add solana
zeroclaw skills add wallet-diligence --bundle solana --edit   # paste SKILL.md
# or copy this directory into <install>/shared/skills/solana/wallet-diligence/
```

Then allow the RPC host for `http_request` and (optionally) set a custom RPC:

```sh
zeroclaw config set http_request.allowed_domains '["api.mainnet-beta.solana.com"]'
# optional: remember a private RPC for higher rate limits
# (the agent reads memory key `solana_rpc_url`)
```

Bind the skill's bundle to your agent and point it at a channel — see
[`../../config/agent.toml`](../../config/agent.toml) and
[`../../SETUP.md`](../../SETUP.md).

## Use

> **check `7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU`**

See [`../../examples/sample_reports.txt`](../../examples/sample_reports.txt) for
real output.
