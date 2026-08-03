//! Offline demo: render a due-diligence report for the built-in demo wallet.
//! No RPC, no keys.
//!
//!   cargo run --example report

fn main() {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "So11111111111111111111111111111111111111112".to_string());
    let now = 1_753_833_600; // fixed for stable output
    println!("{}", solana_wallet_diligence::demo_report(&address, now));
}
