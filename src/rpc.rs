// Off-wasm (unit tests) the RPC methods are unused; they only run in the
// component. Silence the resulting dead-code noise on the host build.
#![allow(dead_code)]

//! Solana JSON-RPC access over `wasi:http` (via `waki`).
//!
//! The Necromancer only *reads* the chain — it exhumes, it never buries. Every
//! call here is a read-only JSON-RPC method against a standard Solana endpoint,
//! so it works against the free `api.mainnet-beta.solana.com` with no API key.

use serde_json::{json, Value};

/// A thin JSON-RPC client bound to one endpoint.
pub struct Rpc {
    url: String,
    id: std::cell::Cell<u64>,
}

impl Rpc {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            id: std::cell::Cell::new(1),
        }
    }

    /// Perform a single JSON-RPC call, returning the `result` field.
    fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.id.get();
        self.id.set(id + 1);
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let bytes = self.transport(&body, method)?;
        let parsed: Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("rpc decode error ({method}): {e}"))?;

        if let Some(err) = parsed.get("error") {
            return Err(format!("rpc returned error ({method}): {err}"));
        }
        Ok(parsed.get("result").cloned().unwrap_or(Value::Null))
    }

    /// The actual wire call. On wasm this uses `waki` over `wasi:http`; natively
    /// (unit tests) there is no transport and it errors — tests exercise the
    /// pure persona pipeline via `mock_remains`, not live RPC.
    #[cfg(target_family = "wasm")]
    fn transport(&self, body: &Value, method: &str) -> Result<Vec<u8>, String> {
        let resp = waki::Client::new()
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(serde_json::to_vec(body).map_err(|e| e.to_string())?)
            .send()
            .map_err(|e| format!("rpc transport error ({method}): {e}"))?;
        resp.body()
            .map_err(|e| format!("rpc body read error ({method}): {e}"))
    }

    #[cfg(not(target_family = "wasm"))]
    fn transport(&self, _body: &Value, _method: &str) -> Result<Vec<u8>, String> {
        Err("no HTTP transport off-wasm (use demo mode or mock_remains)".to_string())
    }

    /// Confirmed signatures for an address, newest first (RPC caps at 1000).
    pub fn signatures_for_address(&self, address: &str, limit: u32) -> Result<Vec<Sig>, String> {
        let limit = limit.clamp(1, 1000);
        let result = self.call(
            "getSignaturesForAddress",
            json!([address, { "limit": limit }]),
        )?;
        let arr = result.as_array().cloned().unwrap_or_default();
        Ok(arr
            .into_iter()
            .map(|v| Sig {
                signature: v.get("signature").and_then(Value::as_str).unwrap_or("").to_string(),
                block_time: v.get("blockTime").and_then(Value::as_i64),
                err: !v.get("err").map(Value::is_null).unwrap_or(true),
            })
            .collect())
    }

    /// Native SOL balance in lamports.
    pub fn balance_lamports(&self, address: &str) -> Result<u64, String> {
        let result = self.call("getBalance", json!([address]))?;
        Ok(result
            .get("value")
            .and_then(Value::as_u64)
            .unwrap_or(0))
    }

    /// SPL token holdings (mint + ui amount) for the owner.
    pub fn token_holdings(&self, address: &str) -> Result<Vec<TokenHolding>, String> {
        const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
        let result = self.call(
            "getTokenAccountsByOwner",
            json!([
                address,
                { "programId": TOKEN_PROGRAM },
                { "encoding": "jsonParsed" }
            ]),
        )?;
        let mut out = Vec::new();
        if let Some(items) = result.get("value").and_then(Value::as_array) {
            for it in items {
                let info = &it["account"]["data"]["parsed"]["info"];
                let mint = info.get("mint").and_then(Value::as_str).unwrap_or("").to_string();
                let ui = info["tokenAmount"]
                    .get("uiAmount")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                if mint.is_empty() {
                    continue;
                }
                out.push(TokenHolding { mint, ui_amount: ui });
            }
        }
        Ok(out)
    }

    /// Program IDs touched by a transaction (top-level instructions only).
    /// Used to profile the wallet's on-chain "personality" from a few samples.
    pub fn tx_program_ids(&self, signature: &str) -> Result<Vec<String>, String> {
        let result = self.call(
            "getTransaction",
            json!([
                signature,
                { "maxSupportedTransactionVersion": 0, "encoding": "jsonParsed" }
            ]),
        )?;
        let mut ids = Vec::new();
        if let Some(instrs) = result["transaction"]["message"]["instructions"].as_array() {
            for ix in instrs {
                if let Some(pid) = ix.get("programId").and_then(Value::as_str) {
                    ids.push(pid.to_string());
                }
            }
        }
        Ok(ids)
    }
}

#[derive(Clone, Debug)]
pub struct Sig {
    pub signature: String,
    pub block_time: Option<i64>,
    pub err: bool,
}

#[derive(Clone, Debug)]
pub struct TokenHolding {
    pub mint: String,
    pub ui_amount: f64,
}
