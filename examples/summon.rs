//! Offline séance demo. Renders the pre-baked demo ghost so you can see exactly
//! what the agent receives — no RPC, no keys, no chain access.
//!
//!   cargo run --example summon
//!   cargo run --example summon -- 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU

fn main() {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "So11111111111111111111111111111111111111112".to_string());
    // Fixed "now" (~2026-07-30) so the demo output is stable.
    let now = 1_753_833_600;
    println!("{}", zeroclaw_necromancer::demo_seance(&address, now));
}
