//! Live due-diligence report via the native reference profiler (ureq → RPC).
//! This is the same analysis the ZeroClaw skill produces from `http_request`;
//! it's here so you can reproduce the sample reports without an agent.
//!
//!   cargo run --example diligence -- <WALLET_ADDRESS> [RPC_URL] [DEPTH]

fn main() {
    let mut args = std::env::args().skip(1);
    let address = match args.next() {
        Some(a) => a,
        None => {
            eprintln!("usage: diligence <WALLET_ADDRESS> [RPC_URL] [DEPTH]");
            std::process::exit(2);
        }
    };
    let rpc = args.next().unwrap_or_else(|| "https://api.mainnet-beta.solana.com".to_string());
    let depth: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1000);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(1_753_833_600);

    match solana_wallet_diligence::live_report(&rpc, &address, depth, 6, now) {
        Ok(report) => println!("{report}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
