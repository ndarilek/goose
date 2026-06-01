//! goose-owned FFI for the iroh ACP tunnel.
//!
//! Exposes a small, goose-shaped surface to Swift (and Kotlin) over UniFFI:
//! connect to a paired goosed over iroh QUIC, open one ACP bidi stream, send
//! newline-delimited JSON-RPC lines, and receive inbound lines via a callback.
//!
//! ACP itself lives in Swift — this crate is only the authenticated byte pipe
//! (iroh: relay + direct path + NodeId identity). No HTTP, no Rust ACP logic.

use std::sync::Arc;

use base64::Engine as _;
use iroh::{endpoint::Endpoint, RelayConfig, RelayMap, RelayMode, RelayUrl, SecretKey};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex};

uniffi::setup_scaffolding!();

const ALPN_GOOSE_ACP_V1: &[u8] = b"goose-acp/1";

const DEFAULT_RELAYS: &[&str] = &[
    "https://usw1-2.relay.michaelneale.mesh-llm.iroh.link./",
    "https://aps1-1.relay.michaelneale.mesh-llm.iroh.link./",
];

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum TunnelError {
    #[error("invalid connection token: {0}")]
    InvalidToken(String),
    #[error("connection failed: {0}")]
    ConnectFailed(String),
    #[error("transport error: {0}")]
    Transport(String),
}

/// Whether traffic is flowing direct (hole-punched) or via a relay.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum PathKind {
    Connecting,
    Direct,
    Relayed,
}

/// Implemented in Swift; receives each inbound newline-delimited JSON-RPC line.
#[uniffi::export(with_foreign)]
pub trait MessageListener: Send + Sync {
    fn on_message(&self, line: String);
    fn on_closed(&self, reason: String);
}

/// Generate a device keypair (hex-encoded 32-byte secret). The NodeId is the
/// public key; persist this in the Keychain so the device identity is stable.
#[uniffi::export]
pub fn generate_device_keypair() -> String {
    let key = SecretKey::generate();
    hex::encode(key.to_bytes())
}

fn relay_mode() -> RelayMode {
    let configs = DEFAULT_RELAYS
        .iter()
        .filter_map(|u| u.parse::<RelayUrl>().ok())
        .map(|url| RelayConfig::new(url, None));
    RelayMode::Custom(RelayMap::from_iter(configs))
}

fn decode_addr(token: &str) -> Result<iroh::EndpointAddr, TunnelError> {
    let json = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|e| TunnelError::InvalidToken(e.to_string()))?;
    serde_json::from_slice(&json).map_err(|e| TunnelError::InvalidToken(e.to_string()))
}

fn device_key(hex_key: &str) -> Result<SecretKey, TunnelError> {
    let bytes = hex::decode(hex_key).map_err(|e| TunnelError::InvalidToken(e.to_string()))?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| TunnelError::InvalidToken("device key must be 32 bytes".into()))?;
    Ok(SecretKey::from_bytes(&arr))
}

/// A connected ACP tunnel: one iroh QUIC bidi stream carrying JSON-RPC lines.
#[derive(uniffi::Object)]
pub struct GooseTunnel {
    runtime: Arc<tokio::runtime::Runtime>,
    send_tx: mpsc::UnboundedSender<String>,
    endpoint: Endpoint,
    server_addr: iroh::EndpointAddr,
    closed: Arc<Mutex<bool>>,
}

#[uniffi::export]
impl GooseTunnel {
    /// Send one newline-delimited JSON-RPC line to the agent.
    pub fn send(&self, line: String) -> Result<(), TunnelError> {
        self.send_tx
            .send(line)
            .map_err(|e| TunnelError::Transport(e.to_string()))
    }

    /// Direct vs relayed, observed from iroh's path info.
    pub fn path_kind(&self) -> PathKind {
        let info = self
            .runtime
            .block_on(async { self.endpoint.remote_info(self.server_addr.id).await });
        match info {
            Some(info) => {
                let has_direct = info
                    .addrs()
                    .any(|a| matches!(a.addr(), iroh::TransportAddr::Ip(_)));
                if has_direct {
                    PathKind::Direct
                } else {
                    PathKind::Relayed
                }
            }
            None => PathKind::Connecting,
        }
    }

    pub fn disconnect(&self) {
        self.runtime.block_on(async {
            *self.closed.lock().await = true;
            self.endpoint.close().await;
        });
    }
}

/// Connect to a paired goosed over iroh and open one ACP stream.
///
/// `server_token` is the base64url `EndpointAddr` from the QR. `device_key_hex`
/// is this device's persisted keypair. Inbound lines are delivered to `listener`.
#[uniffi::export]
pub fn connect(
    server_token: String,
    device_key_hex: String,
    listener: Arc<dyn MessageListener>,
) -> Result<Arc<GooseTunnel>, TunnelError> {
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| TunnelError::Transport(e.to_string()))?,
    );

    let server_addr = decode_addr(&server_token)?;
    let secret = device_key(&device_key_hex)?;
    let (send_tx, mut send_rx) = mpsc::unbounded_channel::<String>();
    let closed = Arc::new(Mutex::new(false));

    let endpoint = runtime
        .block_on(async {
            Endpoint::builder(iroh::endpoint::presets::Minimal)
                .secret_key(secret)
                .alpns(vec![ALPN_GOOSE_ACP_V1.to_vec()])
                .relay_mode(relay_mode())
                .bind()
                .await
        })
        .map_err(|e| TunnelError::ConnectFailed(e.to_string()))?;

    // Connect and open the bidi stream.
    let (mut send, mut recv) = runtime
        .block_on(async {
            let _ =
                tokio::time::timeout(std::time::Duration::from_secs(10), endpoint.online()).await;
            let conn = endpoint
                .connect(server_addr.clone(), ALPN_GOOSE_ACP_V1)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            conn.open_bi()
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))
        })
        .map_err(|e: anyhow::Error| TunnelError::ConnectFailed(e.to_string()))?;

    let closed_writer = closed.clone();
    runtime.spawn(async move {
        while let Some(line) = send_rx.recv().await {
            if *closed_writer.lock().await {
                break;
            }
            let mut buf = line.into_bytes();
            buf.push(b'\n');
            if send.write_all(&buf).await.is_err() {
                break;
            }
            let _ = send.flush().await;
        }
    });

    let listener_reader = listener.clone();
    let closed_reader = closed.clone();
    runtime.spawn(async move {
        let mut buf = Vec::with_capacity(8192);
        let mut chunk = [0u8; 4096];
        loop {
            match recv.read(&mut chunk).await {
                Ok(None) | Ok(Some(0)) | Err(_) => {
                    listener_reader.on_closed("stream ended".into());
                    *closed_reader.lock().await = true;
                    break;
                }
                Ok(Some(n)) => {
                    buf.extend_from_slice(&chunk[..n]);
                    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                        let line: Vec<u8> = buf.drain(..=pos).collect();
                        let line = String::from_utf8_lossy(&line[..line.len() - 1]).to_string();
                        if !line.is_empty() {
                            listener_reader.on_message(line);
                        }
                    }
                }
            }
        }
    });

    Ok(Arc::new(GooseTunnel {
        runtime,
        send_tx,
        endpoint,
        server_addr,
        closed,
    }))
}
