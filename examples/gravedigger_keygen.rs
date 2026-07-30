//! Generate a throwaway "gravedigger" keypair for signing epitaph memos.
//!
//!   cargo run --example gravedigger_keygen
//!
//! Fund the printed PUBKEY with a little SOL (devnet is free), then set
//! `epitaph_signer_key = "<SEED_B58>"` in the plugin's config to enable
//! on-chain epitaphs. The key can ONLY sign memo transactions through this
//! plugin — it can never move funds — but still, use a low-balance wallet.

fn main() {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).expect("OS RNG");
    let seed_b58 = bs58::encode(seed).into_string();
    let pubkey = zeroclaw_necromancer::inscribe::pubkey_from_secret_b58(&seed_b58)
        .expect("derive pubkey");

    eprintln!("🪦 gravedigger keypair (fund with a little SOL for tx fees):\n");
    println!("SEED_B58={seed_b58}");
    println!("PUBKEY={pubkey}");
}
