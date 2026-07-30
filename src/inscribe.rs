//! On-chain epitaph: build and sign a Solana transaction carrying a single SPL
//! **Memo** instruction — the ghost's gravestone, written to the chain.
//!
//! SAFETY: this module can build *only* a memo transaction. There is no code
//! path here that transfers SOL or tokens, invokes an arbitrary program, or adds
//! any instruction other than the one Memo. The worst a leaked/abused signer key
//! can do through this plugin is spend its own lamports on transaction fees.

#![allow(dead_code)]

use ed25519_dalek::{Signer, SigningKey};

/// SPL Memo program (v2).
const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";

/// Decode a signer key from base58. Accepts a 32-byte seed or a 64-byte Solana
/// secret key (seed ‖ pubkey); only the 32-byte seed is used.
fn decode_seed(b58: &str) -> Result<[u8; 32], String> {
    let bytes = bs58::decode(b58.trim())
        .into_vec()
        .map_err(|e| format!("signer key is not valid base58: {e}"))?;
    match bytes.len() {
        32 | 64 => Ok(bytes[..32].try_into().unwrap()),
        n => Err(format!("signer key must be 32 or 64 bytes, got {n}")),
    }
}

fn decode_32(b58: &str, what: &str) -> Result<[u8; 32], String> {
    let bytes = bs58::decode(b58.trim())
        .into_vec()
        .map_err(|e| format!("{what} is not valid base58: {e}"))?;
    bytes
        .try_into()
        .map_err(|_| format!("{what} must decode to 32 bytes"))
}

/// The base58 public key (wallet address) for a signer key.
pub fn pubkey_from_secret_b58(b58: &str) -> Result<String, String> {
    let seed = decode_seed(b58)?;
    let sk = SigningKey::from_bytes(&seed);
    Ok(bs58::encode(sk.verifying_key().to_bytes()).into_string())
}

/// Solana "compact-u16" (shortvec) length prefix.
fn shortvec(len: usize, out: &mut Vec<u8>) {
    let mut n = len;
    loop {
        let mut b = (n & 0x7f) as u8;
        n >>= 7;
        if n != 0 {
            b |= 0x80;
        }
        out.push(b);
        if n == 0 {
            break;
        }
    }
}

/// Build a fully-signed, base58-encoded legacy transaction whose only
/// instruction is an SPL Memo carrying `memo`. `blockhash_b58` is a recent
/// blockhash from `getLatestBlockhash`.
pub fn build_signed_memo_tx(
    secret_b58: &str,
    blockhash_b58: &str,
    memo: &str,
) -> Result<String, String> {
    let seed = decode_seed(secret_b58)?;
    let sk = SigningKey::from_bytes(&seed);
    let payer = sk.verifying_key().to_bytes();
    let memo_prog = decode_32(MEMO_PROGRAM_ID, "memo program id")?;
    let blockhash = decode_32(blockhash_b58, "blockhash")?;

    // ---- message ----
    let mut msg = Vec::new();
    // header: 1 required signature, 0 readonly-signed, 1 readonly-unsigned.
    msg.extend_from_slice(&[1, 0, 1]);
    // account keys: [fee payer (writable signer), memo program (readonly)].
    shortvec(2, &mut msg);
    msg.extend_from_slice(&payer);
    msg.extend_from_slice(&memo_prog);
    // recent blockhash.
    msg.extend_from_slice(&blockhash);
    // instructions: exactly one — the Memo.
    shortvec(1, &mut msg);
    msg.push(1); // program id index → memo program
    shortvec(0, &mut msg); // no account inputs
    let data = memo.as_bytes();
    shortvec(data.len(), &mut msg);
    msg.extend_from_slice(data);

    // ---- sign the message ----
    let sig = sk.sign(&msg).to_bytes();

    // ---- wire transaction: shortvec(sig count) ‖ signature ‖ message ----
    let mut tx = Vec::new();
    shortvec(1, &mut tx);
    tx.extend_from_slice(&sig);
    tx.extend_from_slice(&msg);

    Ok(bs58::encode(tx).into_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    #[test]
    fn memo_tx_is_well_formed_and_correctly_signed() {
        let seed_b58 = bs58::encode([7u8; 32]).into_string();
        let blockhash_b58 = bs58::encode([9u8; 32]).into_string();
        let wire = build_signed_memo_tx(&seed_b58, &blockhash_b58, "here lies a ghost").unwrap();

        let raw = bs58::decode(&wire).into_vec().unwrap();
        assert_eq!(raw[0], 1, "one signature");
        let sig = &raw[1..65];
        let msg = &raw[65..];

        // header + memo payload land where we expect.
        assert_eq!(&msg[0..3], &[1, 0, 1], "message header");
        assert!(msg.ends_with(b"here lies a ghost"), "memo bytes at tail");

        // signature verifies against the derived pubkey over the message bytes.
        let pk_b58 = pubkey_from_secret_b58(&seed_b58).unwrap();
        let pk: [u8; 32] = bs58::decode(&pk_b58).into_vec().unwrap().try_into().unwrap();
        let vk = VerifyingKey::from_bytes(&pk).unwrap();
        let signature = Signature::from_bytes(sig.try_into().unwrap());
        vk.verify(msg, &signature).expect("signature must verify");
    }

    #[test]
    fn rejects_bad_signer_key() {
        assert!(build_signed_memo_tx("not-base58!!", "x", "m").is_err());
    }
}
