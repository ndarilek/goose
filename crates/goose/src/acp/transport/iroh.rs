//! iroh/QUIC transport for ACP.
//!
//! Serves ACP as newline-delimited JSON-RPC directly on an iroh QUIC bidi stream
//! (no HTTP layer). Each accepted bidi stream gets its own agent via
//! `acp::server::serve`, exactly like the stdio and channel-backed transports —
//! only the byte pipe changes.
//!
//! iroh provides: peer discovery (relay + hole-punched direct path), NodeId
//! identity (ed25519 public key, mutually verified by QUIC TLS 1.3), and an
//! authenticated QUIC connection. We bake in default relays so a paired client
//! can find the server behind NAT.

use std::sync::Arc;

use anyhow::{Context, Result};
use iroh::{
    endpoint::{Endpoint, RecvStream, SendStream},
    RelayMap, RelayMode, SecretKey,
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{error, info, warn};

use crate::acp::server_factory::AcpServer;

/// ALPN for goose ACP over QUIC. Bumped if the on-wire ACP framing changes.
pub const ALPN_GOOSE_ACP_V1: &[u8] = b"goose-acp/1";

/// Default relays the client and server share so a phone can find a laptop
/// behind NAT. Overridable via `GOOSE_IROH_RELAYS` (comma-separated URLs).
pub const DEFAULT_RELAYS: &[&str] = &[
    "https://usw1-2.relay.michaelneale.mesh-llm.iroh.link./",
    "https://aps1-1.relay.michaelneale.mesh-llm.iroh.link./",
];

fn effective_relay_urls() -> Vec<String> {
    match std::env::var("GOOSE_IROH_RELAYS") {
        Ok(v) if !v.trim().is_empty() => v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => DEFAULT_RELAYS.iter().map(|s| s.to_string()).collect(),
    }
}

fn relay_mode() -> Result<RelayMode> {
    let urls = effective_relay_urls();
    let configs = urls
        .iter()
        .map(|u| {
            u.parse::<iroh::RelayUrl>()
                .map(|url| iroh::RelayConfig::new(url, None))
                .with_context(|| format!("invalid relay URL: {u}"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(RelayMode::Custom(RelayMap::from_iter(configs)))
}

/// Bind an iroh endpoint for serving ACP, returning the endpoint and the
/// base64url-encoded `EndpointAddr` token a client uses to connect (the QR
/// payload). `secret_key` persists the server's NodeId across restarts.
pub async fn bind_server(secret_key: SecretKey) -> Result<(Endpoint, String)> {
    let endpoint = Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(secret_key)
        .alpns(vec![ALPN_GOOSE_ACP_V1.to_vec()])
        .relay_mode(relay_mode()?)
        .bind()
        .await?;

    // Wait until we're reachable via a relay so the advertised addr is usable.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(10), endpoint.online()).await;

    let addr = endpoint.addr();
    let token = encode_addr_token(&addr)?;
    info!(node_id = %endpoint.id(), "iroh ACP endpoint bound");
    Ok((endpoint, token))
}

pub fn encode_addr_token(addr: &iroh::EndpointAddr) -> Result<String> {
    use base64::Engine as _;
    let json = serde_json::to_vec(addr)?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json))
}

pub fn decode_addr_token(token: &str) -> Result<iroh::EndpointAddr> {
    use base64::Engine as _;
    let json = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .context("invalid endpoint token encoding")?;
    serde_json::from_slice(&json).context("invalid endpoint token JSON")
}

/// Accept loop: each incoming connection may open multiple ACP bidi streams;
/// each stream is served by its own agent.
pub async fn serve(endpoint: Endpoint, server: Arc<AcpServer>) -> Result<()> {
    info!("iroh ACP transport accepting connections");
    while let Some(incoming) = endpoint.accept().await {
        let server = server.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(incoming, server).await {
                warn!("iroh connection ended: {e}");
            }
        });
    }
    Ok(())
}

async fn handle_connection(
    incoming: iroh::endpoint::Incoming,
    server: Arc<AcpServer>,
) -> Result<()> {
    let connection = incoming.await?;
    let remote = connection.remote_id();
    info!(peer = %remote, "iroh ACP connection established");

    loop {
        match connection.accept_bi().await {
            Ok((send, recv)) => {
                let server = server.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(send, recv, server).await {
                        error!("iroh ACP stream error: {e}");
                    }
                });
            }
            Err(e) => {
                info!(peer = %remote, "iroh connection closed: {e}");
                break;
            }
        }
    }
    Ok(())
}

async fn handle_stream(send: SendStream, recv: RecvStream, server: Arc<AcpServer>) -> Result<()> {
    let agent = server.create_agent().await?;
    // QUIC streams are tokio AsyncRead/AsyncWrite; serve() wants futures-io.
    crate::acp::server::serve(agent, recv.compat(), send.compat_write()).await
}
