// `exhume`/`pick_samples` run only inside the wasm component; on the host build
// (unit tests + demo) only `mock_remains` is exercised.
#![allow(dead_code)]

//! Exhumation: gather a wallet's on-chain remains into a single dossier of raw
//! facts. No interpretation happens here — that is the report's job
//! (`persona.rs`). This module just digs.

use crate::rpc::{Rpc, TokenHolding};

/// Everything we could recover about a wallet's on-chain life.
#[derive(Clone, Debug, Default)]
pub struct Remains {
    pub address: String,
    pub total_signatures: usize,
    pub failed_signatures: usize,
    pub first_seen: Option<i64>,
    pub last_seen: Option<i64>,
    pub balance_lamports: u64,
    pub tokens: Vec<TokenHolding>,
    /// Distinct program IDs observed across the sampled transactions.
    pub programs: Vec<String>,
    pub last_signature: Option<String>,
    /// True when the signature scan hit the requested depth — the wallet has
    /// more history than we saw, so `first_seen`/`lifespan_days` are lower
    /// bounds and "very new wallet" must not be inferred.
    pub truncated: bool,
}

impl Remains {
    pub fn sol_balance(&self) -> f64 {
        self.balance_lamports as f64 / 1_000_000_000.0
    }

    /// Non-dust token positions (ui amount > 0).
    pub fn live_tokens(&self) -> usize {
        self.tokens.iter().filter(|t| t.ui_amount > 0.0).count()
    }

    /// Dust / zeroed token accounts — the graveyard of dead memecoins.
    pub fn dust_tokens(&self) -> usize {
        self.tokens.iter().filter(|t| t.ui_amount == 0.0).count()
    }

    /// Days since the wallet's last recorded activity (its "time of death").
    pub fn days_since_death(&self, now_unix: i64) -> Option<i64> {
        self.last_seen.map(|t| ((now_unix - t).max(0)) / 86_400)
    }

    /// Total lifespan in days, from first to last activity.
    pub fn lifespan_days(&self) -> Option<i64> {
        match (self.first_seen, self.last_seen) {
            (Some(a), Some(b)) => Some(((b - a).max(0)) / 86_400),
            _ => None,
        }
    }
}

/// Dig up a wallet. `depth` bounds how many signatures we scan; `samples` bounds
/// how many transactions we fully decode to profile program usage.
pub fn exhume(rpc: &Rpc, address: &str, depth: u32, samples: usize) -> Result<Remains, String> {
    let sigs = rpc.signatures_for_address(address, depth)?;

    if sigs.is_empty() {
        return Err(format!(
            "{address} has no recorded transactions on this cluster — a brand-new, \
             unused, or wrong-network address. Treat as unproven."
        ));
    }

    let total_signatures = sigs.len();
    let failed_signatures = sigs.iter().filter(|s| s.err).count();
    // Signatures come newest-first.
    let last_seen = sigs.first().and_then(|s| s.block_time);
    let first_seen = sigs.iter().rev().find_map(|s| s.block_time);
    let last_signature = sigs.first().map(|s| s.signature.clone());

    let balance_lamports = rpc.balance_lamports(address).unwrap_or(0);
    let tokens = rpc.token_holdings(address).unwrap_or_default();

    // Sample transactions spread across the wallet's life to profile programs
    // without hammering public RPC rate limits.
    let mut programs: Vec<String> = Vec::new();
    for sig in pick_samples(&sigs.iter().map(|s| s.signature.clone()).collect::<Vec<_>>(), samples) {
        if let Ok(ids) = rpc.tx_program_ids(&sig) {
            for id in ids {
                if !programs.contains(&id) {
                    programs.push(id);
                }
            }
        }
    }

    Ok(Remains {
        address: address.to_string(),
        total_signatures,
        failed_signatures,
        first_seen,
        last_seen,
        balance_lamports,
        tokens,
        programs,
        last_signature,
        // We hit the requested window (capped at the RPC's 1000 max) → older
        // history exists beyond what we scanned.
        truncated: total_signatures as u32 >= depth.min(1000),
    })
}

/// Evenly pick up to `n` signatures across a newest-first list so the program
/// profile reflects the whole lifetime, not just the final days.
fn pick_samples(sigs: &[String], n: usize) -> Vec<String> {
    if sigs.is_empty() || n == 0 {
        return Vec::new();
    }
    if sigs.len() <= n {
        return sigs.to_vec();
    }
    let step = sigs.len() as f64 / n as f64;
    (0..n)
        .map(|i| sigs[((i as f64) * step) as usize].clone())
        .collect()
}

/// A pre-baked wallet for offline demos and CI (no RPC needed): a near-empty,
/// dust-laden memecoin trader that should read as ELEVATED risk.
pub fn mock_remains(address: &str) -> Remains {
    Remains {
        address: address.to_string(),
        total_signatures: 4_211,
        failed_signatures: 389,
        // ~ 2021-04-20  →  2022-11-09 (the FTX collapse week)
        first_seen: Some(1_618_920_000),
        last_seen: Some(1_667_952_000),
        balance_lamports: 430_000, // 0.00043 SOL — dust
        tokens: vec![
            TokenHolding { mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".into(), ui_amount: 6_900_000.0 }, // BONK
            TokenHolding { mint: "SAMOxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".into(), ui_amount: 0.0 },
            TokenHolding { mint: "RUGxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".into(), ui_amount: 0.0 },
            TokenHolding { mint: "COPExxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".into(), ui_amount: 0.0 },
        ],
        programs: vec![
            "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".into(),   // Jupiter
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".into(),   // Raydium
            "M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K".into(),    // Magic Eden
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".into(),    // SPL Token
        ],
        last_signature: Some(
            "5xoDe4dGh0stTxSignatureExampleParaNormal1111111111111111111111111111111111111111111111".into(),
        ),
        truncated: true,
    }
}
