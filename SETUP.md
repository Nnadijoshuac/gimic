# Setup — from zero to a running due-diligence bot

Target: a stock ZeroClaw release binary + this skill, on Telegram, in an evening.
Custody tier **T0**: no keys are ever configured.

> Commands follow the ZeroClaw docs (Telegram, Skills, Tools). Run
> `zeroclaw --help` / `zeroclaw config validate` on your version; a few
> enum/nested shapes (cron schedule, peer-group member format) can differ — the
> config file flags those with `⚠VALIDATE`.

## 0. Prerequisites

- A stock ZeroClaw binary (`zeroclaw`) — see the ZeroClaw install docs.
- A model provider key (Anthropic/OpenAI/Ollama/… any of the 70+).
- A Telegram account.

## 1. Model provider

```sh
zeroclaw config set default_provider anthropic
zeroclaw config set default_model claude-sonnet-4-20250514
zeroclaw config set api_key            # masked prompt
```

## 2. Telegram bot

1. In Telegram, open **@BotFather** → `/newbot` → copy the token.
2. Wire it (token is masked, never written to disk in the clear):

```sh
zeroclaw config set channels.telegram.home.bot_token
zeroclaw config set channels.telegram.home.enabled true
```

## 3. Install the skill

```sh
zeroclaw skills bundle add solana
# then copy skills/wallet-diligence/ from this repo into the resolved bundle dir:
#   <install>/shared/skills/solana/wallet-diligence/{SKILL.md,README.md}
zeroclaw skills list --agent primary      # confirm wallet-diligence is loaded
```

## 4. Lock the HTTP tool to the RPC host

```sh
zeroclaw config set http_request.allowed_domains '["api.mainnet-beta.solana.com"]'
```

(Optional) remember a private RPC for higher limits — the skill reads memory key
`solana_rpc_url`:

```sh
zeroclaw memory set solana_rpc_url "https://<your-rpc-host>"
```

## 5. Authorize yourself, bind the agent

Add your Telegram identity to the `owner` peer group (see `config/agent.toml`
and the peer-groups doc for the exact member format), then bind the channel and
bundle to the agent:

```sh
zeroclaw config set agents.primary.channels        # add telegram.home
zeroclaw config set agents.primary.skill_bundles   # add solana
```

## 6. (Optional) watchlist monitor

Copy the `[cron.watchlist]` block from `config/agent.toml`, add `watchlist` to
`agents.primary.cron_jobs`, then `zeroclaw config validate`.

## 7. Run

```sh
zeroclaw daemon           # or: zeroclaw service install && zeroclaw service start
```

DM your bot:

> check 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU

You get a due-diligence report back. Done.

---

## Reproduce the analysis without a full install

The exact risk logic the agent runs is also a native binary and an optional
Tier-3 wasm plugin.

**Native reference profiler** (real RPC, no agent):

```sh
cargo run --example diligence -- 67Pjvywr9R5yadr6hFrnZ4uWqaZSsdxd3SYUsunj5cF9
cargo run --example report            # offline demo wallet, no network
cargo test                            # risk-logic unit tests
```

**Optional Tier-3 plugin** (sandboxed wasm component, run via a tiny wasmtime host):

```sh
cargo build --release --target wasm32-wasip2
cd tools/live-host && cargo run --release -- <WALLET_ADDRESS>
```
