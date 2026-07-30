//! # ZeroClaw Necromancer 🦴
//!
//! A Solana-native ZeroClaw tool plugin that **summons and channels the ghost
//! of any wallet.** Give it an address; it exhumes the wallet's entire on-chain
//! afterlife (transaction history, holdings, the programs it haunted), performs
//! a "digital autopsy" to determine cause of death, raises a personality from
//! the remains, and hands the agent a directive to *speak as the dead wallet.*
//!
//! It is strictly read-only. The Necromancer robs graves; it never digs them.

// These three modules are pure logic (the RPC transport call is target-gated
// inside `rpc`), so the whole persona pipeline is unit-testable natively while
// the WIT `component` below is compiled only for wasm.
mod exhume;
mod persona;
mod rpc;

/// Render a séance for `address` using the offline **demo ghost** (no RPC).
/// Exposed so the `summon` example and docs can show authentic output with no
/// network access. Live summoning happens inside the wasm `component` module.
pub fn demo_seance(address: &str, now_unix: i64) -> String {
    let remains = exhume::mock_remains(address);
    let ghost = persona::raise(&remains, now_unix);
    persona::render_seance(&remains, &ghost, now_unix)
}

#[cfg(target_family = "wasm")]
mod component {
    use crate::{exhume, persona, rpc::Rpc};
    use serde_json::Value;

    wit_bindgen::generate!({
        path: "wit",
        world: "necromancer",
        features: ["plugins-wit-v0"],
        // Our root package (`necromancer:seance`) re-exports the zeroclaw
        // interfaces from a dependency package, so emit bindings for all
        // referenced interfaces, not just the root's own.
        generate_all,
    });

    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};

    const DEFAULT_RPC: &str = "https://api.mainnet-beta.solana.com";

    struct Necromancer;

    impl PluginInfo for Necromancer {
        fn plugin_name() -> String {
            "zeroclaw-necromancer".to_string()
        }
        fn plugin_version() -> String {
            env!("CARGO_PKG_VERSION").to_string()
        }
    }

    impl Tool for Necromancer {
        fn name() -> String {
            "seance".to_string()
        }

        fn description() -> String {
            "Summon and channel the ghost of a Solana wallet. Given a wallet \
             address, it exhumes the wallet's on-chain history (transactions, \
             SOL and token holdings, the programs it interacted with), performs \
             a digital autopsy to infer its 'cause of death' and personality, \
             and returns a séance readout ending with a directive instructing \
             you to role-play AS that departed wallet's ghost. Use it when the \
             user wants to 'talk to', 'summon', 'channel', or investigate the \
             life of any Solana address. Read-only."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "address": {
                        "type": "string",
                        "description": "The Solana wallet address (base58) to summon."
                    },
                    "depth": {
                        "type": "integer",
                        "description": "How many recent signatures to exhume (1-1000).",
                        "default": 300,
                        "minimum": 1,
                        "maximum": 1000
                    },
                    "samples": {
                        "type": "integer",
                        "description": "How many transactions to fully decode to profile the wallet's personality.",
                        "default": 5,
                        "minimum": 1,
                        "maximum": 20
                    },
                    "demo": {
                        "type": "boolean",
                        "description": "Summon a pre-baked demo ghost with no RPC access (for testing).",
                        "default": false
                    }
                },
                "required": ["address"]
            })
            .to_string()
        }

        fn execute(args: String) -> Result<ToolResult, String> {
            let v: Value = serde_json::from_str(&args)
                .map_err(|e| format!("invalid tool arguments: {e}"))?;

            let address = v
                .get("address")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            if !looks_like_solana_address(&address) {
                return Ok(fail(format!(
                    "'{address}' does not look like a Solana address. The spirits \
                     require a valid base58 pubkey (32–44 chars)."
                )));
            }

            let depth = v.get("depth").and_then(Value::as_u64).unwrap_or(300) as u32;
            let samples = v.get("samples").and_then(Value::as_u64).unwrap_or(5) as usize;

            // Config arrives injected under `__config` (only when the plugin is
            // granted the `ConfigRead` permission). `demo` may be set via args
            // or config.
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

            let ghost = persona::raise(&remains, now);
            let seance = persona::render_seance(&remains, &ghost, now);

            Ok(ToolResult {
                success: true,
                output: seance,
                error: None,
            })
        }
    }

    fn fail(msg: impl Into<String>) -> ToolResult {
        let msg = msg.into();
        ToolResult {
            success: false,
            output: format!("🕯️ The séance failed.\n\n{msg}"),
            error: Some(msg),
        }
    }

    /// Wall-clock unix seconds, or a fixed fallback if the host denies clocks.
    fn now_unix() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(1_753_800_000) // ~2025-07-29, a stable fallback
    }

    /// Cheap sanity check: base58 alphabet, plausible pubkey length. The RPC
    /// rejects anything that slips through, so this only needs to be a filter.
    fn looks_like_solana_address(s: &str) -> bool {
        const B58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        (32..=44).contains(&s.len()) && s.bytes().all(|b| B58.contains(&b))
    }

    export!(Necromancer);
}

#[cfg(test)]
mod tests {
    use crate::exhume::{mock_remains, Remains};
    use crate::persona;
    use crate::rpc::TokenHolding;

    // A fixed "now" so day-count assertions are stable: ~2024-06-01.
    const NOW: i64 = 1_717_200_000;

    #[test]
    fn demo_ghost_renders_a_full_seance() {
        let r = mock_remains("So11111111111111111111111111111111111111112");
        let g = persona::raise(&r, NOW);
        let s = persona::render_seance(&r, &g, NOW);

        assert!(s.contains("R.I.P."), "tombstone present");
        assert!(s.contains("CHANNELING DIRECTIVE"), "directive present");
        assert!(s.contains("first person"), "directive tells agent to role-play");
        // The demo ghost is a dust-balance memecoin casualty.
        assert!(g.cause_of_death.contains("Gas Starvation") || g.cause_of_death.contains("Rug"));
    }

    #[test]
    fn spirit_name_is_deterministic() {
        let addr = "AbCdEfGhJkLmNpQrStUvWxYz123456789ABCDEFGH";
        let a = persona::raise(&mock_remains(addr), NOW).spirit_name;
        let b = persona::raise(&mock_remains(addr), NOW).spirit_name;
        assert_eq!(a, b, "same wallet must always raise the same ghost");
    }

    #[test]
    fn recent_activity_reads_as_undead() {
        let mut r = Remains {
            address: "So11111111111111111111111111111111111111112".into(),
            total_signatures: 10,
            ..Default::default()
        };
        r.first_seen = Some(NOW - 86_400 * 400);
        r.last_seen = Some(NOW - 86_400 * 3); // died 3 days ago
        r.balance_lamports = 2_000_000_000;
        let g = persona::raise(&r, NOW);
        assert!(g.undead, "activity within 30 days is undead");
        assert!(g.cause_of_death.contains("UNDEAD"));
    }

    #[test]
    fn diamond_hands_retire_in_peace() {
        let mut r = Remains::default();
        r.address = "So11111111111111111111111111111111111111112".into();
        r.total_signatures = 500;
        r.first_seen = Some(NOW - 86_400 * 1000);
        r.last_seen = Some(NOW - 86_400 * 400); // long dead
        r.balance_lamports = 12_000_000_000; // 12 SOL, untouched
        r.programs = vec!["MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD".into()];
        let g = persona::raise(&r, NOW);
        assert!(g.cause_of_death.contains("Peaceful Retirement"), "got: {}", g.cause_of_death);
    }

    #[test]
    fn dead_wallet_with_no_history_is_rejected_upstream() {
        // Empty remains would never be raised; exhume() guards it. Here we just
        // confirm token accounting helpers behave.
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
