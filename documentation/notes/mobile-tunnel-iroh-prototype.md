# ACP-over-iroh mobile tunnel — working prototype

> Status: **end-to-end proven** on 2026-06-01. Pure-Swift client → goose-owned
> FFI → iroh QUIC (relay + direct) → goosed ACP, both direct and relay paths.
> All changes are local prototype code; see "What's prototype-quality" below.

## What was built

### 1. Server: iroh ACP transport in goose
`crates/goose/src/acp/transport/iroh.rs` (new) — binds an iroh `Endpoint`
(ALPN `goose-acp/1`), registers with the two baked-in relays
(`*.relay.michaelneale.mesh-llm.iroh.link`, override via `GOOSE_IROH_RELAYS`),
accepts QUIC connections, and serves **ACP as newline-delimited JSON-RPC directly
on each bidi stream** via the existing `acp::server::serve` (Option A — no HTTP
layer). Also encodes/decodes the `EndpointAddr` connection token (the QR payload).

Launch: `crates/goose-cli/src/cli.rs` — `goose serve --iroh`. Persists a stable
NodeId in `~/.config/goose/iroh_acp_secret.key`, prints the NodeId + base64url
connection token.

### 2. Client FFI: goose-owned, not iroh-ffi
`crates/goose-tunnel-ffi/` (new crate) — a small UniFFI surface, **not** n0's broad
`iroh-ffi`/`IrohLib`. Exposes only:
- `generate_device_keypair() -> String` (hex; NodeId = the device identity)
- `connect(server_token, device_key_hex, listener) -> GooseTunnel`
- `GooseTunnel.send(line)` / `.path_kind() -> Direct|Relayed|Connecting` / `.disconnect()`
- `trait MessageListener { on_message(line); on_closed(reason) }`

Internally: iroh client `Endpoint`, relays baked in, opens one ACP bidi stream,
newline-frames JSON-RPC both directions. **ACP logic stays in the (Swift) caller** —
the FFI is just the authenticated byte pipe.

### 3. Swift driver (end-to-end proof, in-repo)
`crates/goose-tunnel-ffi/swift-driver/{main.swift,run.sh}` — pure Swift, links the
generated UniFFI bindings + the dylib. Generates a device key, connects via a token,
sends ACP `initialize`, parses the response. `run.sh` regenerates bindings, compiles,
and runs in one step. (A proof harness; the real iOS app would reuse its existing ACP
client over the same FFI.)

## What was verified (live)

```
# direct path (same LAN — token has Relay + Ip):
🛣️  path: direct
➡️  {"jsonrpc":"2.0","id":1,"method":"initialize",...}
⬅️  {"jsonrpc":"2.0","result":{"protocolVersion":1,"agentCapabilities":{...}}}
✅ ACP initialize round-trip succeeded over iroh   (exit 0)

# relay path (off-network sim — token stripped to Relay only):
🛣️  path: relayed
⬅️  {"jsonrpc":"2.0","result":{...}}                (exit 0)
```

- **Direct path:** iroh hole-punched a direct QUIC path on the LAN automatically.
- **Relay path:** with a relay-only token (the real off-network mobile case),
  traffic flowed through the relay — E2E encrypted, relay sees only ciphertext.
- **Same ACP, both paths:** identical `initialize` round-trip; real goose agent
  capabilities returned.
- **Identity:** QUIC TLS 1.3 mutually authenticates NodeIds; the device key is the
  client's identity (pin it on pairing for authorization — not yet enforced, below).

## How to reproduce
```bash
# 1. build
cargo build -p goose-cli --bin goose -p goose-tunnel-ffi

# 2. server — prints NodeId + connection token
./target/debug/goose serve --iroh

# 3. run the Swift driver end to end (regenerates bindings, compiles, runs)
crates/goose-tunnel-ffi/swift-driver/run.sh "<connection-token>"
```
Verified reproducing from a clean tree on 2026-06-01: direct path (LAN) and relay
path (relay-only token) both exit 0 with a real ACP `initialize` round-trip.

## Mobile lifecycle testing (tested like a phone, not just a CLI)

A plain end-to-end run is a desktop-style process. What makes it *mobile* is the
lifecycle: suspend kills the UDP socket, foreground must reconnect fast, and the
path may be relay or direct. These were exercised with two extra in-repo harnesses:

- `swift-driver/mobile-lifecycle.swift` (run via `run-mobile.sh`) — **3
  foreground/background cycles** with a **stable device identity** (generated once,
  reused on every reconnect, like a Keychain-persisted key). Each foreground:
  connect → ACP `initialize` → clean `disconnect()` (simulating iOS background
  teardown). **Result: PASS** — every cycle reconnects cold and re-establishes ACP
  in ~1.3–2.7s.
- `swift-driver/path-probe.swift` — connect with a **relay-only token** (the true
  off-network/cellular case: client is given only the relay, no LAN address) and
  sample `path_kind()` over time.

### What this proved (honestly)
- **Suspend/resume works:** cold reconnect + ACP re-init succeeds repeatedly with a
  stable NodeId. This is the core mobile loop.
- **Relay path is real:** with a relay-only token the path stays `relayed` for the
  whole session and ACP completes over the relay (E2E encrypted, relay sees only
  ciphertext).
- **Direct path is real:** with the full token (relay + LAN IP) iroh hole-punches to
  `direct` automatically.

### Honest single-host caveat
Client and server run on **one machine**, so a loopback direct path always exists.
After a relay-bootstrapped connection, iroh's disco can learn that loopback
candidate and upgrade `relayed → direct` on later reconnects — correct iroh
behavior, but it means a *sustained* relay-only path can't be guaranteed on one
host. A truly permanent relay path (no direct possible) needs two machines / real
NAT / cellular, or network-level blocking of the direct path. The relay path itself
is proven (probe + cycle #1); its permanence under real NAT is not single-host
testable.

### Not yet tested (needs a real device / Xcode app)
- iOS `scenePhase`/`NWPathMonitor`-driven reconnect (here it's simulated by explicit
  disconnect/reconnect).
- QUIC connection migration on a live Wi-Fi↔cellular switch.
- Behavior across a real OS process freeze (vs. simulated teardown).
- Streaming `session/prompt` interrupted mid-turn by suspend + resume cursor.

## What's prototype-quality (not production)

1. **No device authorization yet.** Any client that has the token can connect
   (transport identity is verified, but we don't pin/allow-list the device NodeId
   on goosed). Next: capture device NodeId at QR-pairing, enforce an allow-list.
2. **One stream per connection, no resume cursor.** Suspend/resume (§4 of the
   research note) — event-id cursor + `resume(session, after_event=N)` — not built.
3. **Token strips to relay-only manually** in the test; real client just uses the
   full token and lets iroh choose (verified it does).
4. **FFI runs its own tokio runtime per connect** — fine for one tunnel; revisit
   if multiple.
5. **Server reuses `acp::server::serve` per stream**, bypassing the
   `ConnectionRegistry`/replay buffer. To get replay-on-reconnect, route through the
   registry instead (small change; the registry is already transport-agnostic).
6. **iOS lifecycle glue not built** (no Xcode app here) — `scenePhase` +
   `NWPathMonitor` → reconnect, `beginBackgroundTask` clean-close. Design in §5 of
   the research note.
7. **xcframework packaging not done** — bindings generated for macOS dylib only;
   the iOS build pipeline (lipo + create-xcframework for arm64/sim) is the
   `make_swift.sh`-style step from §8.
8. **iroh `1.0.0-rc.1`** pinned in both goose and the FFI crate — keep them in lock
   step (shared relay/disco wire protocol).

## Files changed
- `crates/goose/src/acp/transport/iroh.rs` (new)
- `crates/goose/src/acp/transport/mod.rs` (+`pub mod iroh;`)
- `crates/goose/Cargo.toml` (+`iroh`)
- `crates/goose-cli/src/cli.rs` (`--iroh` flag, `handle_serve_iroh_command`, key persistence)
- `crates/goose-cli/Cargo.toml` (+`iroh`, `hex`)
- `crates/goose-tunnel-ffi/` (new crate: `Cargo.toml`, `src/lib.rs`, `src/uniffi-bindgen.rs`)
- `crates/goose-tunnel-ffi/swift-driver/` (in-repo Swift harnesses + run scripts:
  `main.swift`/`run.sh` = basic e2e; `mobile-lifecycle.swift`/`run-mobile.sh` =
  suspend/resume cycles; `path-probe.swift` = relay-vs-direct path observation)
