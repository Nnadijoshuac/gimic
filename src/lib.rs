//! # Solana Wallet Due-Diligence
//!
//! The read-only engine behind the ZeroClaw **wallet-diligence** skill: given a
//! Solana address, it reads the wallet's on-chain history (transactions, SOL and
//! token holdings, the programs it used) and produces a compact, plain-English
//! **counterparty risk report** — a usage profile, concrete risk signals, and a
//! LOW / CAUTION / ELEVATED verdict.
//!
//! **Custody tier T0 (read-only).** Nothing here holds a key, signs, sends, or
//! mutates anything on-chain. There is no transaction-building code path at all.
//!
//! On a stock ZeroClaw binary this logic lives in a skill (`skills/wallet-diligence/`)
//! driven by the built-in `http_request` tool — no compiled code required. This
//! crate is the optional Tier-3 variant: the same profiler as a sandboxed
//! `wasm32-wasip2` tool plugin, and the native reference implementation used to
//! generate the sample reports.

mod exhume;
mod report;
mod rpc;

/// Render a due-diligence report for `address` using the offline **demo wallet**
/// (no RPC). Exposed for the `report` example and docs.
pub fn demo_report(address: &str, now_unix: i64) -> String {
    let remains = exhume::mock_remains(address);
    let r = report::analyze(&remains, now_unix);
    report::render(&r)
}

/// Run a live due-diligence report natively (used by the `diligence` example and
/// the sample-report generator). Native only — the wasm component path lives in
/// `component` below.
#[cfg(not(target_family = "wasm"))]
pub fn live_report(rpc_url: &str, address: &str, depth: u32, samples: usize, now_unix: i64) -> Result<String, String> {
    let remains = exhume::exhume(&rpc::Rpc::new(rpc_url), address, depth, samples)?;
    Ok(report::render(&report::analyze(&remains, now_unix)))
}

#[cfg(target_family = "wasm")]
mod component {
    use crate::{exhume, report, rpc::Rpc};
    use serde_json::Value;

    wit_bindgen::generate!({
        path: "wit",
        world: "diligence",
        features: ["plugins-wit-v0"],
        generate_all,
    });

    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};

    const DEFAULT_RPC: &str = "https://api.mainnet-beta.solana.com";

    struct Diligence;

    impl PluginInfo for Diligence {
        fn plugin_name() -> String {
            "solana-wallet-diligence".to_string()
        }
        fn plugin_version() -> String {
            env!("CARGO_PKG_VERSION").to_string()
        }
    }

    impl Tool for Diligence {
        fn name() -> String {
            "wallet_diligence".to_string()
        }

        fn description() -> String {
            "Read-only Solana wallet due-diligence. Given a wallet address, reads \
             its on-chain history (transactions, SOL and token holdings, programs \
             used) and returns a compact counterparty risk report: a usage \
             profile, concrete risk signals, and a LOW / CAUTION / ELEVATED \
             verdict. Use it to vet a counterparty before an OTC trade, a P2P \
             deal, or a treasury payment. It never signs or sends anything."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "address": { "type": "string", "description": "The Solana wallet address (base58) to vet." },
                    "depth": { "type": "integer", "description": "How many recent signatures to scan (1-1000).", "default": 300, "minimum": 1, "maximum": 1000 },
                    "samples": { "type": "integer", "description": "How many transactions to decode to fingerprint program usage.", "default": 5, "minimum": 1, "maximum": 20 },
                    "demo": { "type": "boolean", "description": "Use a built-in demo wallet with no RPC access (for testing).", "default": false }
                },
                "required": ["address"]
            })
            .to_string()
        }

        fn execute(args: String) -> Result<ToolResult, String> {
            let v: Value = serde_json::from_str(&args)
                .map_err(|e| format!("invalid tool arguments: {e}"))?;

            let address = v.get("address").and_then(Value::as_str).unwrap_or("").trim().to_string();
            if !looks_like_solana_address(&address) {
                return Ok(fail(format!(
                    "'{address}' is not a valid Solana address (expected base58, 32–44 chars)."
                )));
            }

            let depth = v.get("depth").and_then(Value::as_u64).unwrap_or(300) as u32;
            let samples = v.get("samples").and_then(Value::as_u64).unwrap_or(5) as usize;

            let cfg = v.get("__config");
            let demo = v.get("demo").and_then(Value::as_bool).unwrap_or(false)
                || cfg
                    .and_then(|c| c.get("demo"))
                    .and_then(Value::as_str)
                    .map(|s| s == "true" || s == "1")
                    .unwrap_or(false);

            let now = now_unix();
            let remains = if demo {
                exhume::mock_remains(&address)
            } else {
                let rpc_url = cfg
                    .and_then(|c| c.get("rpc_url"))
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .unwrap_or(DEFAULT_RPC);
                match exhume::exhume(&Rpc::new(rpc_url), &address, depth, samples) {
                    Ok(r) => r,
                    Err(e) => return Ok(fail(e)),
                }
            };

            let output = report::render(&report::analyze(&remains, now));
            Ok(ToolResult { success: true, output, error: None })
        }
    }

    fn fail(msg: impl Into<String>) -> ToolResult {
        let msg = msg.into();
        ToolResult { success: false, output: format!("Due-diligence failed: {msg}"), error: Some(msg) }
    }

    fn now_unix() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(1_753_800_000)
    }

    fn looks_like_solana_address(s: &str) -> bool {
        const B58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        (32..=44).contains(&s.len()) && s.bytes().all(|b| B58.contains(&b))
    }

    export!(Diligence);
}

#[cfg(test)]
mod tests {
    use crate::exhume::{mock_remains, Remains};
    use crate::report::{self, Level};
    use crate::rpc::TokenHolding;

    const NOW: i64 = 1_717_200_000; // ~2024-06-01

    #[test]
    fn demo_wallet_reports_elevated_with_dust() {
        // The demo wallet is a near-empty, dust-laden memecoin trader.
        let r = report::analyze(&mock_remains("So11111111111111111111111111111111111111112"), NOW);
        assert_eq!(r.level, Level::Elevated, "signals: {:?}", r.risk_signals);
        let text = report::render(&r);
        assert!(text.contains("Wallet due-diligence"));
        assert!(text.contains("ELEVATED"));
        assert!(text.contains("→ "), "has a recommendation line");
    }

    #[test]
    fn clean_established_wallet_reads_low() {
        let mut r = Remains::default();
        r.address = "5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9".into();
        r.total_signatures = 800;
        r.failed_signatures = 12; // ~1.5%
        r.first_seen = Some(NOW - 86_400 * 900);
        r.last_seen = Some(NOW - 86_400 * 3);
        r.balance_lamports = 25_000_000_000; // 25 SOL
        r.programs = vec![
            "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".into(),
            "MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD".into(),
        ];
        let rep = report::analyze(&r, NOW);
        assert_eq!(rep.level, Level::Low, "signals: {:?}", rep.risk_signals);
    }

    #[test]
    fn brand_new_wallet_is_flagged() {
        let mut r = Remains::default();
        r.address = "5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9".into();
        r.total_signatures = 6;
        r.first_seen = Some(NOW - 86_400 * 2);
        r.last_seen = Some(NOW - 3600);
        r.balance_lamports = 2_000_000_000;
        let rep = report::analyze(&r, NOW);
        assert!(matches!(rep.level, Level::Caution | Level::Elevated));
        assert!(rep.risk_signals.iter().any(|s| s.contains("new wallet")));
    }

    #[test]
    fn no_history_is_insufficient() {
        let mut r = Remains::default();
        r.address = "5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9".into();
        r.total_signatures = 2;
        let rep = report::analyze(&r, NOW);
        assert_eq!(rep.level, Level::Insufficient);
    }

    #[test]
    fn token_accounting() {
        let r = Remains {
            tokens: vec![
                TokenHolding { mint: "a".into(), ui_amount: 0.0 },
                TokenHolding { mint: "b".into(), ui_amount: 5.0 },
            ],
            ..Default::default()
        };
        assert_eq!(r.live_tokens(), 1);
        assert_eq!(r.dust_tokens(), 1);
    }
}
