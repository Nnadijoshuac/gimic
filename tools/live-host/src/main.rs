//! Minimal wasmtime host that loads the compiled wallet-diligence component and
//! runs its `wallet_diligence` tool against a live Solana RPC — mirroring
//! ZeroClaw's own v45 plugin loading path (base WASI p2 + gated wasi:http + the
//! tool-plugin world). Lets anyone reproduce the Tier-3 plugin without a full
//! ZeroClaw install.
//!
//! Usage: necromancer-live-host <wallet_address> [rpc_url] [depth]
//!
//! Build the component first:
//!   (cd ../.. && cargo build --release --target wasm32-wasip2)

use anyhow::{Context, Result};
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_http::WasiHttpCtx;
use wasmtime_wasi_http::p2::{WasiHttpCtxView, WasiHttpView};

mod bindings {
    wasmtime::component::bindgen!({
        world: "diligence",
        path: "../../wit",
        imports: { default: async },
        exports: { default: async },
    });
}
use bindings::Diligence;
use bindings::zeroclaw::plugin::logging::{Host as LoggingHost, LogLevel, PluginEvent};

struct State {
    table: ResourceTable,
    wasi: WasiCtx,
    http: WasiHttpCtx,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView { ctx: &mut self.wasi, table: &mut self.table }
    }
}

impl WasiHttpView for State {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: wasmtime_wasi_http::p2::default_hooks(),
        }
    }
}

// `types` is a pure type-alias interface; bindgen still wants an (empty) Host.
impl bindings::zeroclaw::plugin::types::Host for State {}

// The component tree-shook logging away, but the world declares the import, so
// the host still provides it. No-op is fine.
impl LoggingHost for State {
    async fn log_record(&mut self, _level: LogLevel, _event: PluginEvent) {}
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let address = args.next().context("usage: necromancer-live-host <wallet_address> [rpc_url] [depth]")?;
    let rpc_url = args.next().unwrap_or_else(|| "https://api.mainnet-beta.solana.com".to_string());
    let depth: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1000);

    let wasm_path = std::env::var("NECRO_WASM").unwrap_or_else(|_| {
        "../../target/wasm32-wasip2/release/solana_wallet_diligence.wasm".into()
    });

    let mut config = Config::new();
    config.async_support(true);
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;

    let component = Component::from_file(&engine, &wasm_path)
        .map_err(|e| anyhow::anyhow!("loading component at {wasm_path}: {e}"))?;

    let mut linker: Linker<State> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    wasmtime_wasi_http::p2::add_only_http_to_linker_async(&mut linker)?;
    let mut options = bindings::LinkOptions::default();
    options.plugins_wit_v0(true);
    Diligence::add_to_linker::<_, HasSelf<_>>(&mut linker, &options, |s| s)?;

    let mut store = Store::new(
        &engine,
        State {
            table: ResourceTable::new(),
            wasi: WasiCtx::builder().build(),
            http: WasiHttpCtx::new(),
        },
    );

    let plugin = Diligence::instantiate_async(&mut store, &component, &linker).await?;
    let tool = plugin.zeroclaw_plugin_tool();

    let input = serde_json::json!({
        "address": address,
        "depth": depth,
        "samples": 6,
        "__config": { "rpc_url": rpc_url }
    })
    .to_string();

    eprintln!("⏳ vetting {address} via {rpc_url} …\n");
    match tool.call_execute(&mut store, &input).await? {
        Ok(result) => {
            println!("{}", result.output);
            if !result.success {
                eprintln!("\n(tool reported success=false)");
            }
        }
        Err(e) => {
            eprintln!("execute error: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}
