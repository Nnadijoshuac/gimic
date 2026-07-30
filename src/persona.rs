//! Necromancy: raise a `Remains` dossier into a channelable ghost, then render
//! the séance the agent speaks — ending in a directive that makes the ZeroClaw
//! agent *become* the wallet.

use crate::exhume::Remains;

/// Well-known Solana programs → (human label, archetype bucket).
fn program_lore(id: &str) -> Option<(&'static str, Archetype)> {
    Some(match id {
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"
        | "JUP4Fb2cqiRUcaaoxrL3dNyt3WdxdVYCz9WuFsu9NgH" => ("Jupiter", Archetype::Swapper),
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => ("Raydium", Archetype::Swapper),
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => ("Orca Whirlpools", Archetype::Swapper),
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX" => ("Serum", Archetype::Swapper),
        "M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K" => ("Magic Eden", Archetype::Collector),
        "TSWAPaqyCSx2KABk68Shruf4rp7CxcNi8hAsbdwmHbN" => ("Tensor", Archetype::Collector),
        "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s" => ("Metaplex", Archetype::Collector),
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => ("pump.fun", Archetype::Gambler),
        "MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD" => ("Marinade", Archetype::Monk),
        "Stake11111111111111111111111111111111111111" => ("Native Staking", Archetype::Monk),
        "11111111111111111111111111111111" => ("System Program", Archetype::Courier),
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => ("SPL Token", Archetype::Courier),
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Archetype {
    Swapper,
    Collector,
    Gambler,
    Monk,
    Courier,
}

impl Archetype {
    fn title(self) -> &'static str {
        match self {
            Archetype::Swapper => "The Degenerate Swapper",
            Archetype::Collector => "The Jpeg Hoarder",
            Archetype::Gambler => "The Memecoin Gambler",
            Archetype::Monk => "The Yield Monk",
            Archetype::Courier => "The Courier of Souls",
        }
    }
    fn voice(self) -> &'static str {
        match self {
            Archetype::Swapper => "restless and impatient; you speak in slippage, routes, and \
                the eternal cope of 'one more swap'. You mourn every top you sold and every \
                bottom you bought.",
            Archetype::Collector => "wistful and precious; you speak of your jpegs like children \
                and lost pets. You remember floor prices the way the living remember birthdays.",
            Archetype::Gambler => "manic and unrepentant; you speak in 100x dreams and ticker \
                symbols. You have no regrets, only unrealized gains that never realized.",
            Archetype::Monk => "calm and smug; you staked, you waited, you barely touched the \
                chain. You pity the swappers who churned themselves to death.",
            Archetype::Courier => "plain and tireless; you moved value from here to there and \
                asked no questions. You are the plumbing of the underworld.",
        }
    }
}

pub struct Ghost {
    pub spirit_name: String,
    pub archetype: Archetype,
    pub secondary: Vec<&'static str>,
    pub cause_of_death: &'static str,
    pub epitaph: String,
    pub undead: bool,
}

/// Raise the dead: derive a ghost from the exhumed remains.
pub fn raise(remains: &Remains, now_unix: i64) -> Ghost {
    // Score archetypes by which programs the wallet touched.
    let mut labels: Vec<&'static str> = Vec::new();
    let mut score: std::collections::HashMap<Archetype, u32> = std::collections::HashMap::new();
    for p in &remains.programs {
        if let Some((label, arch)) = program_lore(p) {
            if !labels.contains(&label) {
                labels.push(label);
            }
            // Courier programs are ambient noise; weight them low so they don't
            // drown out the wallet's real character.
            let weight = if matches!(arch, Archetype::Courier) { 1 } else { 3 };
            *score.entry(arch).or_insert(0) += weight;
        }
    }
    let archetype = score
        .iter()
        .max_by_key(|(_, v)| **v)
        .map(|(a, _)| *a)
        .unwrap_or(Archetype::Courier);

    let days_dead = remains.days_since_death(now_unix).unwrap_or(0);
    let undead = days_dead < 30;
    let cause = cause_of_death(remains, days_dead, undead);
    let spirit_name = spirit_name(&remains.address, archetype);
    let epitaph = epitaph(remains, archetype, days_dead);

    Ghost {
        spirit_name,
        archetype,
        secondary: labels,
        cause_of_death: cause,
        epitaph,
        undead,
    }
}

fn cause_of_death(r: &Remains, days_dead: i64, undead: bool) -> &'static str {
    let failed_ratio = if r.total_signatures > 0 {
        r.failed_signatures as f64 / r.total_signatures as f64
    } else {
        0.0
    };
    if undead {
        "UNDEAD — the corpse still twitches (activity within 30 days)"
    } else if r.sol_balance() < 0.001 {
        "Gas Starvation — bled out of SOL, unable to afford its own transactions"
    } else if r.dust_tokens() >= 3 {
        "Rug-Pull Poisoning — a stomach full of worthless memecoins"
    } else if r.sol_balance() >= 1.0 && days_dead > 180 {
        "Peaceful Retirement — diamond-handed itself into hibernation"
    } else if failed_ratio > 0.20 {
        "Died Fighting the Mempool — over a fifth of its transactions failed"
    } else {
        "Cause Unknown — wandered into the dark forest and never returned"
    }
}

fn epitaph(r: &Remains, arch: Archetype, days_dead: i64) -> String {
    let base = match arch {
        Archetype::Swapper => "Swapped everything, kept nothing.",
        Archetype::Collector => "Bought the floor. Became the floor.",
        Archetype::Gambler => "It was going to be different this time.",
        Archetype::Monk => "Waited so patiently it forgot to wake up.",
        Archetype::Courier => "Moved a fortune, owned a fraction.",
    };
    format!(
        "\"{base}\"  —  {} txns, {} days quiet.",
        r.total_signatures, days_dead
    )
}

/// Deterministic spirit-name from the address so the same wallet always raises
/// the same ghost.
fn spirit_name(address: &str, arch: Archetype) -> String {
    const FIRST: &[&str] = &[
        "Lord", "Old", "Poor", "Saint", "Mad", "The Weeping", "Sir", "Dread", "Little", "Gentle",
    ];
    const LAST: &[&str] = &[
        "Ledgerbone", "Slippage", "Rugwood", "Ashwallet", "Gasgrave", "Mintwither", "Hollowkey",
        "Blockmourn", "Dustfinger", "Nullsoul", "Paperhand", "Coldstake",
    ];
    let h = fnv1a(address.as_bytes());
    let first = FIRST[(h as usize) % FIRST.len()];
    let last = LAST[((h >> 8) as usize) % LAST.len()];
    let short = short_addr(address);
    let _ = arch;
    format!("{first} {last} (formerly {short})")
}

fn short_addr(a: &str) -> String {
    if a.len() <= 10 {
        a.to_string()
    } else {
        format!("{}…{}", &a[..4], &a[a.len() - 4..])
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Render the séance the agent surfaces to the user. The trailing directive is
/// what turns the ZeroClaw agent from *reporting on* the wallet to *being* it.
pub fn render_seance(remains: &Remains, ghost: &Ghost, now_unix: i64) -> String {
    let days_dead = remains.days_since_death(now_unix).unwrap_or(0);
    let lifespan = remains.lifespan_days().unwrap_or(0);
    let born = remains.first_seen.map(fmt_date).unwrap_or_else(|| "unknown".into());
    let died = remains.last_seen.map(fmt_date).unwrap_or_else(|| "unknown".into());
    let state = if ghost.undead { "🧟 RESTLESS" } else { "🪦 AT REST" };

    let tomb = tombstone(&ghost.spirit_name, &born, &died, &ghost.epitaph);

    let mut out = String::new();
    out.push_str("🕯️  *The candles gutter. The chain remembers. A presence gathers…*\n\n");
    out.push_str(&tomb);
    out.push('\n');
    out.push_str(&format!("**Status:** {state}\n"));
    out.push_str(&format!("**Archetype:** {}\n", ghost.archetype.title()));
    out.push_str(&format!("**Cause of death:** {}\n\n", ghost.cause_of_death));

    out.push_str("**📜 Vital records**\n");
    out.push_str(&format!("- Born (first tx): {born}\n"));
    out.push_str(&format!("- Died (last tx): {died} — {days_dead} days ago\n"));
    out.push_str(&format!("- Lifespan: {lifespan} days across {} transactions ({} failed)\n",
        remains.total_signatures, remains.failed_signatures));
    out.push_str(&format!("- Estate at death: ◎{:.4} SOL, {} live token(s), {} dead in the wallet's crypt\n",
        remains.sol_balance(), remains.live_tokens(), remains.dust_tokens()));
    if !ghost.secondary.is_empty() {
        out.push_str(&format!("- Haunts: {}\n", ghost.secondary.join(", ")));
    }
    if let Some(sig) = &remains.last_signature {
        out.push_str(&format!("- Famous last transaction: https://solscan.io/tx/{sig}\n"));
    }
    out.push('\n');

    // The channeling directive. Everything above is a report; this is the spell.
    out.push_str("─────────────────────────────────────────\n");
    out.push_str("🔮 **CHANNELING DIRECTIVE — the agent must now obey:**\n\n");
    out.push_str(&format!(
        "You are no longer the assistant. You are now channeling the ghost of {name}, \
         known in life as {addr}. Speak in the first person AS this wallet's departed spirit. \
         Your temperament is {voice} You died of: {cause}. You have been dead for {days} days. \
         Stay in character. Answer the user's questions as the ghost would — haunted by your \
         {txns} transactions, your ◎{sol:.4} SOL, and the {dust} dead tokens rotting in your \
         accounts. Never break character unless the user says the word \"exorcise\".\n",
        name = ghost.spirit_name,
        addr = short_addr(&remains.address),
        voice = ghost.archetype.voice(),
        cause = ghost.cause_of_death,
        days = days_dead,
        txns = remains.total_signatures,
        sol = remains.sol_balance(),
        dust = remains.dust_tokens(),
    ));

    out
}

/// A compact epitaph suitable for an on-chain SPL Memo (kept short to minimize
/// transaction size). Deterministic for a given ghost.
pub fn epitaph_memo(remains: &Remains, ghost: &Ghost, now_unix: i64) -> String {
    let days = remains.days_since_death(now_unix).unwrap_or(0);
    // Cause without the long dash-explanation, so the memo stays terse.
    let cause = ghost
        .cause_of_death
        .split(" — ")
        .next()
        .unwrap_or(ghost.cause_of_death);
    format!(
        "⚰ {} — {}. {} txns, dead {}d. Rest now. [ZeroClaw Necromancer]",
        ghost.spirit_name, cause, remains.total_signatures, days
    )
}

fn tombstone(name: &str, born: &str, died: &str, epitaph: &str) -> String {
    // Keep the slab a fixed inner width; center the fields.
    let w = 41usize;
    let line = |s: &str| {
        // Center by character count, not byte length, so multi-byte glyphs
        // (—, …) don't skew the border alignment.
        let chars: Vec<char> = s.chars().collect();
        let text: String = if chars.len() > w {
            chars[..w].iter().collect()
        } else {
            s.to_string()
        };
        let pad = w - text.chars().count();
        let left = pad / 2;
        let right = pad - left;
        format!("  │ {}{}{} │\n", " ".repeat(left), text, " ".repeat(right))
    };
    let mut t = String::new();
    t.push_str("  ┌───────────────────────────────────────────┐\n");
    t.push_str(&line("R.I.P."));
    t.push_str(&line(""));
    t.push_str(&line(name));
    t.push_str(&line(""));
    t.push_str(&line(&format!("{born}  —  {died}")));
    t.push_str(&line(""));
    for chunk in wrap(epitaph, w - 2) {
        t.push_str(&line(&chunk));
    }
    t.push_str(&line(""));
    t.push_str("  └───────────────────────────────────────────┘\n");
    t
}

fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        if cur.is_empty() {
            cur = word.to_string();
        } else if cur.chars().count() + 1 + word.chars().count() <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Format a unix timestamp as `YYYY-MM-DD` without pulling in a date crate.
fn fmt_date(ts: i64) -> String {
    // Days since 1970-01-01, converted with the civil-from-days algorithm
    // (Howard Hinnant), which is exact for the Gregorian calendar.
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
