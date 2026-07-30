//! Build a signed epitaph-memo transaction and print it as base58 wire bytes —
//! the exact bytes the plugin broadcasts. Handy for `simulateTransaction`.
//!
//!   cargo run --example build_epitaph_tx -- <SEED_B58> <RECENT_BLOCKHASH_B58> "memo text"

fn main() {
    let mut a = std::env::args().skip(1);
    let seed = a.next().expect("usage: build_epitaph_tx <seed> <blockhash> [memo]");
    let blockhash = a.next().expect("need a recent blockhash");
    let memo = a.next().unwrap_or_else(|| "epitaph".to_string());
    match zeroclaw_necromancer::inscribe::build_signed_memo_tx(&seed, &blockhash, &memo) {
        Ok(wire) => println!("{wire}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
