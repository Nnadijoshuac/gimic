//! Turn a wallet's exhumed on-chain remains into a compact, plain-English
//! **due-diligence report**: a usage profile, concrete risk signals, and a
//! LOW / CAUTION / ELEVATED verdict a human can act on before transacting with
//! a counterparty. Deterministic and read-only.

use crate::exhume::Remains;

/// Well-known Solana programs → (label, category). Used to fingerprint how a
/// wallet actually behaves on-chain.
fn program_lore(id: &str) -> Option<(&'static str, Category)> {
    Some(match id {
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"
        | "JUP4Fb2cqiRUcaaoxrL3dNyt3WdxdVYCz9WuFsu9NgH" => ("Jupiter", Category::Dex),
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => ("Raydium", Category::Dex),
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => ("Orca", Category::Dex),
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX" => ("Serum", Category::Dex),
        "M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K" => ("Magic Eden", Category::Nft),
        "TSWAPaqyCSx2KABk68Shruf4rp7CxcNi8hAsbdwmHbN" => ("Tensor", Category::Nft),
        "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s" => ("Metaplex", Category::Nft),
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => ("pump.fun", Category::Memecoin),
        "MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD" => ("Marinade", Category::Staking),
        "Stake11111111111111111111111111111111111111" => ("Native Staking", Category::Staking),
        "11111111111111111111111111111111" => ("System", Category::Transfer),
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => ("SPL Token", Category::Transfer),
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Category {
    Dex,
    Nft,
    Memecoin,
    Staking,
    Transfer,
}

impl Category {
    fn profile_label(self) -> &'static str {
        match self {
            Category::Dex => "active DEX trader",
            Category::Nft => "NFT collector / trader",
            Category::Memecoin => "memecoin speculator",
            Category::Staking => "staker / long-term holder",
            Category::Transfer => "transfers-only / operational wallet",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Low,
    Caution,
    Elevated,
    Insufficient,
}

impl Level {
    pub fn badge(self) -> &'static str {
        match self {
            Level::Low => "✅ LOW RISK",
            Level::Caution => "🟡 CAUTION",
            Level::Elevated => "⚠️ ELEVATED RISK",
            Level::Insufficient => "❔ INSUFFICIENT HISTORY",
        }
    }
}

pub struct Report {
    pub address: String,
    pub level: Level,
    pub confidence: &'static str,
    pub first_seen: Option<i64>,
    pub last_seen: Option<i64>,
    pub days_since: i64,
    pub lifespan_days: i64,
    pub total: usize,
    pub failed_pct: u32,
    pub sol: f64,
    pub live_tokens: usize,
    pub dust_tokens: usize,
    pub profile_label: &'static str,
    pub programs: Vec<&'static str>,
    pub risk_signals: Vec<String>,
    pub positives: Vec<String>,
    pub recommendation: &'static str,
    pub truncated: bool,
}

/// Analyze exhumed remains into a due-diligence report.
pub fn analyze(remains: &Remains, now_unix: i64) -> Report {
    // Program fingerprint → dominant profile.
    let mut labels: Vec<&'static str> = Vec::new();
    let mut score: std::collections::HashMap<Category, u32> = std::collections::HashMap::new();
    let mut has_memecoin = false;
    for p in &remains.programs {
        if let Some((label, cat)) = program_lore(p) {
            if !labels.contains(&label) {
                labels.push(label);
            }
            if matches!(cat, Category::Memecoin) {
                has_memecoin = true;
            }
            let weight = if matches!(cat, Category::Transfer) { 1 } else { 3 };
            *score.entry(cat).or_insert(0) += weight;
        }
    }
    let dominant = score
        .iter()
        .max_by_key(|(_, v)| **v)
        .map(|(c, _)| *c)
        .unwrap_or(Category::Transfer);

    let days_since = remains.days_since_death(now_unix).unwrap_or(0);
    let lifespan_days = remains.lifespan_days().unwrap_or(0);
    let failed_pct = if remains.total_signatures > 0 {
        ((remains.failed_signatures as f64 / remains.total_signatures as f64) * 100.0).round() as u32
    } else {
        0
    };
    let sol = remains.sol_balance();
    let dust = remains.dust_tokens();
    let truncated = remains.truncated;

    // ── risk signals ──
    let mut risk = Vec::new();
    let mut positives = Vec::new();

    if remains.total_signatures < 5 {
        risk.push("Almost no on-chain history — unproven counterparty.".to_string());
    } else if truncated {
        // We hit the scan window: the wallet is busier/older than we saw. A
        // fresh scam wallet does not accumulate this much history, so this is
        // mildly reassuring — but note the age is unknown.
        positives.push(format!(
            "High activity — more than {} transactions (older/busier than this scan window).",
            remains.total_signatures
        ));
    } else if lifespan_days < 14 {
        risk.push(format!(
            "Very new wallet ({lifespan_days}d of history) — limited track record."
        ));
    } else if lifespan_days >= 180 {
        positives.push(format!("Established wallet ({lifespan_days}d of history)."));
    }

    if sol < 0.001 && remains.total_signatures >= 5 {
        risk.push("Near-zero SOL — cannot cover its own fees (drained or abandoned?).".to_string());
    } else if sol >= 1.0 {
        positives.push(format!("Holds ◎{sol:.2} SOL."));
    }

    if dust >= 5 {
        risk.push(format!(
            "{dust} empty token accounts — heavy airdrop/spam exposure (common in farm/scam-adjacent wallets)."
        ));
    }

    if failed_pct >= 20 {
        risk.push(format!(
            "High failed-transaction rate ({failed_pct}%) — bot-like or erratic."
        ));
    } else if remains.total_signatures >= 50 && failed_pct <= 5 {
        positives.push(format!("Clean execution ({failed_pct}% failed txns)."));
    }

    if has_memecoin {
        risk.push("Interacts with memecoin-launch programs (pump.fun) — high speculative risk.".to_string());
    }

    if days_since >= 180 && remains.total_signatures >= 5 {
        risk.push(format!("Dormant {days_since}d — stale; confirm the wallet is still controlled."));
    }

    // ── verdict ──
    let level = if remains.total_signatures < 5 {
        Level::Insufficient
    } else if risk.len() >= 2 {
        Level::Elevated
    } else if risk.len() == 1 {
        Level::Caution
    } else {
        Level::Low
    };

    let confidence = if truncated {
        "partial (history truncated at 1000 signatures)"
    } else if remains.total_signatures < 20 {
        "low (thin history)"
    } else {
        "good"
    };

    let recommendation = match level {
        Level::Elevated => {
            "Treat as high-risk. Verify identity out-of-band and avoid custody-heavy interactions."
        }
        Level::Caution => "Proceed with care; confirm the counterparty through a second channel.",
        Level::Low => "No major on-chain red flags (this is a heuristic, not a guarantee).",
        Level::Insufficient => "Not enough history to assess — do not rely on this wallet's reputation.",
    };

    Report {
        address: remains.address.clone(),
        level,
        confidence,
        first_seen: remains.first_seen,
        last_seen: remains.last_seen,
        days_since,
        lifespan_days,
        total: remains.total_signatures,
        failed_pct,
        sol,
        live_tokens: remains.live_tokens(),
        dust_tokens: dust,
        profile_label: dominant.profile_label(),
        programs: labels,
        risk_signals: risk,
        positives,
        recommendation,
        truncated,
    }
}

/// Render a compact report (kept small on purpose — a chat-resident agent must
/// not flood its context with raw RPC).
pub fn render(r: &Report) -> String {
    let born = r.first_seen.map(fmt_date).unwrap_or_else(|| "unknown".into());
    let seen = r.last_seen.map(fmt_date).unwrap_or_else(|| "unknown".into());
    let mut out = String::new();
    out.push_str(&format!("🔍 Wallet due-diligence — {}\n", short_addr(&r.address)));
    out.push_str(&format!("Verdict: {}  (confidence: {})\n\n", r.level.badge(), r.confidence));

    if r.truncated {
        out.push_str(&format!(
            "• Scanned: last {} txns (window; full age not shown), {}% failed\n",
            r.total, r.failed_pct
        ));
    } else {
        out.push_str(&format!(
            "• History: {} txns over {}→{} ({}d), {}% failed\n",
            r.total, born, seen, r.lifespan_days, r.failed_pct
        ));
    }
    out.push_str(&format!(
        "• Last activity: {} ({} days ago){}\n",
        seen,
        r.days_since,
        if r.days_since >= 180 { " — dormant" } else { "" }
    ));
    out.push_str(&format!(
        "• Holdings: ◎{:.3} SOL, {} live tokens, {} empty accounts\n",
        r.sol, r.live_tokens, r.dust_tokens
    ));
    out.push_str(&format!("• Profile: {}", r.profile_label));
    if !r.programs.is_empty() {
        out.push_str(&format!(" (uses {})", r.programs.join(", ")));
    }
    out.push('\n');

    if !r.risk_signals.is_empty() {
        out.push_str("\n⚠️ Risk signals:\n");
        for s in &r.risk_signals {
            out.push_str(&format!("  - {s}\n"));
        }
    }
    if !r.positives.is_empty() {
        out.push_str("\n✅ Reassuring:\n");
        for s in &r.positives {
            out.push_str(&format!("  - {s}\n"));
        }
    }
    out.push_str(&format!("\n→ {}\n", r.recommendation));
    out
}

fn short_addr(a: &str) -> String {
    if a.len() <= 10 {
        a.to_string()
    } else {
        format!("{}…{}", &a[..4], &a[a.len() - 4..])
    }
}

/// Format a unix timestamp as `YYYY-MM-DD` (civil-from-days, Howard Hinnant).
fn fmt_date(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}
