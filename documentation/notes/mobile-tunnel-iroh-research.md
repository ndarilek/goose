# Replacing the goose Mobile Tunnel with ACP over iroh/QUIC

> Research note — exploratory, no implementation. Proposes throwing out the
> "lapstone" HTTP-over-WebSocket tunnel and replacing it with **ACP carried over
> iroh QUIC streams**, reachable from a **pure-Swift iOS app via a goose-owned FFI
> framework** (`Goose.xcframework`, built with the n0 `iroh-ffi` *recipe* but not
> shipping their broad binding — no Rust source in the app). Framing decision: ACP
> as newline-delimited JSON-RPC directly on the QUIC stream (Option A, §2).

## 0. Corrections to earlier assumptions (read first)

Two things I previously got wrong, now verified:

1. **iroh *does* have an official Swift binding.** `github.com/n0-computer/iroh-ffi`
   ships:
   - a **prebuilt `Iroh.xcframework`** (`ios-arm64`, `ios-arm64_x86_64-simulator`,
     `macos-arm64`),
   - an **`IrohLib` Swift Package** + CocoaPods podspecs (`IrohLib.podspec`,
     `IrohLibFramework.podspec`),
   - a documented Swift usage guide (`README.swift.md`) and a
     "build-your-own-binding" route (docs.iroh.computer/deployment/other-languages).
   The binding is UniFFI-generated. So Swift consumes iroh as an **opaque binary
   framework** — no Rust toolchain or Rust source in the app project.

2. **You don't have to reimplement the relay/QUIC protocol, and you don't have to
   author the FFI from scratch.** `iroh-ffi`/`IrohLib` is a reference/example with a
   far-too-broad surface (blobs/docs/gossip) — we **don't ship it** (§8). Instead we
   reuse its *build recipe* to produce a small **goose-owned** `Goose.xcframework`
   exposing just connect + ACP stream + cancel, with our relays baked in.

The plain-QUIC-gateway idea (previous "Option A") is **dropped**: a gateway that
terminates the phone's QUIC is just lapstone with a nicer transport — no direct
path, no end-to-end encryption. Not worth doing.

## 1. What exists today (to be replaced)

- **lapstone tunnel** (`crates/goose-server/src/tunnel/{mod.rs,lapstone.rs}`,
  `routes/tunnel.rs`): goosed dials an outbound WebSocket to a **personal
  Cloudflare Worker**; the iOS app hits that Worker over HTTPS. Every phone HTTP
  request is reframed as JSON `TunnelMessage`/`TunnelResponse` over WS, with manual
  chunking and SSE-streaming handling.
- **The mobile client talks HTTP+SSE, not ACP.** It hits goosed's REST surface —
  notably `POST /reply` which returns `text/event-stream`
  (`routes/reply.rs`: `SseResponse`, `Content-Type: text/event-stream`). The tunnel
  is a generic HTTP proxy; the phone speaks the same HTTP API the desktop does.
- **ACP already has network transports.** Beyond stdio, goose has a
  transport-agnostic ACP server at **`crates/goose/src/acp/transport/`**:
  `http.rs`, `websocket.rs`, `connection.rs`, `mod.rs`. It serves ACP over an axum
  router on `/acp` — `POST` (JSON-RPC request), `GET` (WebSocket upgrade *or* SSE),
  `DELETE` (teardown), scoped by `Acp-Connection-Id` / `Acp-Session-Id` headers.
  **This is the integration point — not stdio.**

### The transport abstraction (this is what we hook into)
`connection.rs` defines a transport-agnostic `Connection` + `ConnectionRegistry`:
- `to_agent_tx: mpsc::Sender<String>` — client→agent JSON-RPC lines.
- An outbound fan-out (`OutboundStream`, broadcast) for agent→client, **with a
  pre-subscribe replay buffer** (`subscribe_with_replay`): messages emitted before
  a subscriber attaches are buffered and replayed on (re)subscribe.
- `adapters.rs` bridges mpsc ↔ `AsyncRead/AsyncWrite` with newline JSON-RPC framing.

A concrete transport (`websocket::run_ws`) is **~70 lines**: split the socket,
replay buffered messages, then `select!` { socket→`to_agent_tx` ; `outbound_rx`→
socket }. **An iroh transport is the same ~70 lines over a QUIC bidi stream** — the
`Connection`, registry, session routing, and replay buffer are all reused unchanged.
The replay buffer is also **most of the suspend/resume story already** (reconnect →
replay); we only extend the cursor to survive a full reconnect (§4).

### Why replace it
- Single personal relay = SPOF + trust anchor; **Cloudflare terminates TLS and
  sees plaintext** (no E2E).
- **Never a direct path** — even same-Wi-Fi traffic round-trips through the Worker.
- Auth is a **shared 32-byte bearer secret** in the QR (plus a buggy
  `secure_compare` using non-constant-time 64-bit `DefaultHasher` — fix regardless).
- Bespoke HTTP-over-JSON-over-WS reframing split across 3 repos; binary is lossy.

## 2. Target architecture

```
iOS app (pure Swift)         Goose.xcframework (goose-owned, vendored)    laptop
┌────────────────┐  goose-    ┌───────────────────────────┐  iroh QUIC  ┌────────┐
│ SwiftUI + ACP  │  tunnel    │ iroh Endpoint (client)    │ ──relay──►  │ goosed │
│ client (Swift) │  Swift API │ ALPN goose-acp/1          │ ◄─direct──► │  +iroh │
└────────────────┘ ─────────► │ 2 relays baked in         │             └────────┘
        no Rust source        └───────────────────────────┘   E2E encrypted; relay
                                                                sees ciphertext only
```

- **goosed** binds an iroh `Endpoint` (ALPN `goose-acp/1`), registers with the two
  relays via `RelayMode::Custom`, runs an accept loop, and serves **ACP as
  newline-delimited JSON-RPC on each accepted QUIC bidi stream** (Option A below).
- **iOS app** uses the goose-owned `Goose.xcframework` (§8) to dial the server's
  `NodeAddr`, open a bidi QUIC stream, and speak the **same newline JSON-RPC ACP it
  already uses** — only the byte pipe changes. The two relays are compiled-in defaults.
- On LAN: connects via relay, then **hole-punches to a direct path** automatically.
  Off-net: rides the relay, **end-to-end encrypted** (relay can't read traffic).

**Server change is small because ACP transports are already pluggable.** goosed adds
a `crates/goose/src/acp/transport/iroh.rs` next to `websocket.rs`: bind an iroh
`Endpoint` (ALPN `goose-acp/1`, 2 relays via `RelayMode::Custom`), accept loop →
for each QUIC bidi stream call `registry.create_connection()` and run the same
`select!` bridge as `run_ws`. No new ACP semantics, no protocol redesign — the
agent loop, session routing, and replay buffer are reused verbatim.

### The layer stack — and the one real decision

iroh is **not** a peer of HTTP. These are stacked layers; the only open choice is
the *framing* layer:

```
┌──────────────────────────────────────────────────────────────┐
│ ACP        JSON-RPC 2.0 (initialize, session/prompt, …)        │  the protocol
├──────────────────────────────────────────────────────────────┤
│ FRAMING    newline-delimited JSON-RPC   ← THE choice (Option A) │  ← decided: A
│            (alt: HTTP/3 over the top — rejected, Option B)      │
├──────────────────────────────────────────────────────────────┤
│ STREAM     a reliable, ordered, bidi byte stream               │
│            == iroh QUIC bidi stream (SendStream/RecvStream)     │
├──────────────────────────────────────────────────────────────┤
│ QUIC       streams + TLS 1.3 + multiplexing + migration        │  (inside iroh)
├──────────────────────────────────────────────────────────────┤
│ iroh       discover peer + NodeId identity + relay/holepunch    │  finds + authenticates
│            → hands you an authenticated QUIC connection         │  → a QUIC connection
└──────────────────────────────────────────────────────────────┘
```

**iroh's job:** find the goose server (relay or direct), authenticate it by NodeId,
and produce an authenticated QUIC connection. **ACP's job:** ride a byte stream as
JSON-RPC lines. A QUIC bidi stream *is* the byte stream — they meet directly.

### DECISION: Option A — ACP as newline-delimited JSON-RPC directly on the QUIC stream

**No HTTP layer.** This is not a close call:

1. **Zero impedance with the agent core.** `acp::server::serve(agent, read, write)`
   already consumes a generic `AsyncRead`/`AsyncWrite` of newline-delimited JSON-RPC
   — exactly what stdio uses. A QUIC `SendStream`/`RecvStream` *is* that pair, so the
   QUIC stream feeds `serve()` directly (via the existing `ReceiverToAsyncRead`/
   `compat` adapters). Option B would stand up an HTTP server on the QUIC connection
   only to re-derive the same byte stream the agent then re-parses.
2. **HTTP framing solves problems we no longer have.** The `/acp` POST/GET-as-SSE/
   DELETE routes + `Acp-*` headers + chunking + content-type sniffing exist to
   smuggle a long-lived **bidirectional** JSON-RPC conversation through request/
   response HTTP. A QUIC bidi stream is natively bidirectional and long-lived —
   HTTP-over-the-top re-introduces the very SSE/chunking machinery we're deleting.
3. **Smallest, lowest-risk change.** One file, `transport/iroh.rs`, mirroring
   `websocket.rs`'s ~70-line `select!` loop over QUIC streams. `Connection`,
   `ConnectionRegistry`, session routing, and the replay buffer reused verbatim.
   No HTTP/3 server, no route re-mounting, no SSE plumbing.
4. **Proven shape.** mesh-llm runs its protocol as frames straight on iroh QUIC
   streams (no HTTP) — same shape as ACP-on-QUIC.
5. **Cleaner suspend/resume.** Owning the framing end-to-end lets us add a resume
   cursor (event-id on notifications, "resume after N" on reconnect) as a protocol
   decision we control, instead of leaning on HTTP/SSE `Last-Event-ID` semantics and
   gateway-style buffering (§4).

**Framing sub-choice:** use **newline-delimited JSON-RPC** (identical to stdio + WS
today → maximum reuse). Length-prefixed frames (mesh-llm style) are marginally more
robust for large/binary payloads but unnecessary for line-oriented JSON ACP — don't
add unless a concrete need appears.

**Option B (HTTP/3 over QUIC) — rejected.** Only justifiable to reuse the exact
existing `/acp` HTTP handlers unchanged for a throwaway prototype; the transport
abstraction is already thin enough that the saving is tiny and the HTTP layer is
permanent overhead. Not worth it.

Net: **one protocol, one transport.** Mobile speaks ACP over an iroh QUIC stream;
desktop/CLI keep their existing transports. The HTTP REST surface (`/reply`, the
`/acp` HTTP routes) stays for the desktop, but mobile no longer depends on it.

## 3. Identity & end-to-end auth (replaces the shared secret)

- **Transport identity is free from iroh/QUIC.** Each endpoint is an ed25519
  keypair; **NodeId == public key**, mutually verified by QUIC's TLS 1.3 handshake.
  goosed learns the phone's verified public key; the phone learns goosed's. No
  bearer secret needed to prove *who* you're talking to.
- **Authorization = per-device NodeId pinning.** At QR-pairing time, capture the
  phone's NodeId into an allow-list in goose config. Only enrolled devices connect;
  revoke one device without rotating a global secret. (mesh-llm's
  `SignedNodeOwnership` certs over NodeIds are a ready blueprint if we want signed
  enrollment rather than a plain allow-list.)
- **Pairing QR** carries `base64url(NodeAddr)` = server NodeId + relay URLs
  (iroh's `EndpointAddr`/`NodeAddr` token, cf. mesh-llm
  `encode/decode_endpoint_addr_token`), replacing `goosechat://configure?{url,secret}`.
- Local `server_secret` can still gate the loopback bridge as defense-in-depth; it
  never leaves the machine.

## 4. Operational behavior on iOS — the suspend/resume lifecycle

**Fundamental constraint (transport-independent):** iroh runs over UDP on a tokio
runtime *inside* the xcframework. When iOS suspends the app it **freezes the
process** — tokio stops scheduling, keepalives stop, the relay session and any
direct path hit their idle timeout and are reaped server-side. **No entitlement
keeps a general data app's UDP socket alive** in the background (VoIP/PushKit is
for calls; Apple rejects misuse). Design rule: **don't survive suspension —
survive *resumption* fast.**

| Phase | Behavior | Action |
|---|---|---|
| Foreground, LAN | relay first → **hole-punch to direct QUIC** in ~1-2s | observe via `home_relay()`/`remote_info_list()` |
| Foreground, off-net | relay path, **ciphertext-only** through relay | normal operation |
| Backgrounding | ~seconds before freeze | **clean `disconnect()`** in `beginBackgroundTask`; persist stream cursor; mark `Suspended` |
| Frozen | nothing runs | fine — don't fight it |
| Foreground resume | re-`connect()`; QUIC **0-RTT session resume (~ms)**; auto direct-path re-upgrade | trigger on `scenePhase == .active` |
| Roaming (Wi-Fi↔cellular, foregrounded) | QUIC **connection migration**, often seamless (no reconnect) | trigger on `NWPathMonitor` change |
| Mid-stream across suspend | stream dies (any transport) | **app-level resume cursor** (see below) |
| Wake while asleep | impossible over iroh (frozen phone can't receive UDP) | needs **APNs push** to foreground first |

**vs. lapstone:** background-drop is the same (unavoidable on iOS), but resume is
**0-RTT QUIC vs. full TCP+TLS+WS reconnect**, roaming can be **seamless migration
vs. full reconnect**, and foreground gives **direct path + true E2E** that lapstone
never has.

### Streaming continuity across suspend (iroh does NOT solve this)
A QUIC stream carrying an in-flight ACP turn dies on freeze. Design ACP-over-iroh
with a **resume cursor**:
- goosed tags each `session/update` with a monotonic event id.
- On reconnect the client sends `resume(session_id, after_event=N)`.
- goosed replays buffered tail events, or returns the completed turn if it finished
  while the app was away.
This is the same problem lapstone has today; it's an application-protocol concern.

### "Notify me when it's done while asleep"
Only achievable with **APNs**: a relay-side or goosed-side hook fires a push → app
foregrounds → iroh reconnects (0-RTT) → client pulls the result via resume cursor.
Orthogonal to transport; the only path to asleep-delivery on iOS.

## 5. Minimal Swift lifecycle glue (sketch)

```swift
// One "ensure connected" routine driven by both lifecycle and network triggers.
@MainActor final class GooseTunnel: ObservableObject {
    @Published var path: PathKind = .disconnected   // .direct / .relayed / .suspended

    private let node: GooseTunnelNode     // from goose-owned Goose.xcframework (§8)
    private var conn: GooseAcpStream?
    private var resumeCursor: UInt64 = 0
    private let serverAddr: NodeAddr      // decoded from paired QR (base64url)

    func ensureConnected() async {
        guard conn == nil else { return }
        conn = try? await node.connectAcp(serverAddr,
                                          resumeAfter: resumeCursor)   // 0-RTT when possible
        path = (try? await node.isDirect(serverAddr)) == true ? .direct : .relayed
    }

    func onForeground() { Task { await ensureConnected() } }        // scenePhase == .active

    func onPathChange(_ p: NWPath) {                                 // NWPathMonitor
        // QUIC migration usually handles this transparently when foregrounded;
        // only re-dial if the stream actually dropped.
        if conn == nil { Task { await ensureConnected() } }
    }

    func onBackground(_ task: UIBackgroundTaskID) {                  // beginBackgroundTask
        conn?.closeCleanly()                                         // don't leave a zombie
        conn = nil
        path = .suspended
        // resumeCursor already persisted as updates arrive
    }
}
```
The `connectAcp` / `isDirect` / `closeCleanly` / event-id cursor methods are what
the **goose-owned FFI surface** (`crates/goose-tunnel-ffi`, §8) exposes. Internally
it uses iroh's `Endpoint` + `node_addr`/`home_relay`/`remote_info_list` to decide
direct-vs-relayed and to open the ACP bidi stream — but the Swift app only sees the
small goose-shaped API, not iroh's full surface.

## 6. Implementation shape (when we act)

**Server (Rust, in goose) — small, because ACP transports are pluggable:**
1. Add **`crates/goose/src/acp/transport/iroh.rs`** next to `websocket.rs`: bind iroh
   `Endpoint` (ALPN `goose-acp/1`, `RelayMode::Custom` with the two baked-in relays),
   accept loop → per QUIC bidi stream `registry.create_connection()` + the same
   `select!` bridge `run_ws` already uses. **Reuses `Connection`, registry, session
   routing, and the replay buffer verbatim** — no new ACP semantics.
2. Wire it into goosed startup (where `tunnel/mod.rs` is today): bind endpoint,
   carry over the watchdog/reconnect + single-instance lock.
3. Device enrollment: NodeId allow-list in goose config; QR emits `base64url(NodeAddr)`.
4. Extend the replay buffer with a **persistent per-session event cursor** so a full
   reconnect (post-suspend) resumes instead of replaying-from-attach only (§4).
5. Delete lapstone (`tunnel/lapstone.rs`, Cloudflare Worker dependency,
   `tunnel_secret` plumbing, `secure_compare`).

**Client (Swift app) — talks the *same* ACP it already speaks, just over iroh:**
1. Vendor the goose-owned `Goose.xcframework` (§8) — exposes connect + ACP stream.
2. The existing Swift ACP/JSON-RPC client logic is reused; only the byte transport
   changes (iroh bidi stream instead of the HTTP/WS-via-Worker it uses today).
3. Lifecycle glue (§5): `scenePhase` + `NWPathMonitor` → `ensureConnected`.
4. QR pairing → store `NodeAddr` + device keypair (Keychain).
5. (Later) APNs for asleep-delivery.

**Relays:**
- Run our own iroh relays (cf. mesh-llm's `usw1-2 / aps1-1.relay.…iroh.link`),
  fall back to n0 default relays, make configurable — mirror `effective_relay_urls`.

## 7. Trade-offs summary

| Concern | lapstone (today) | ACP over iroh |
|---|---|---|
| LAN datapath | always via Cloudflare | **direct (hole-punched)** |
| Off-net datapath | via Cloudflare (plaintext) | via relay (**ciphertext only**) |
| E2E encryption to app | no | **yes** |
| Identity/auth | shared 32-byte secret | **per-device ed25519 NodeId, mutually verified** |
| Relay trust | personal Worker, SPOF | self-hostable iroh relays, can't read traffic |
| Protocol | HTTP→JSON→WS reframe + manual chunk/stream | **native ACP over QUIC bidi stream** |
| Binary payloads | lossy (UTF-8 strings) | native bytes |
| Swift client | HTTP via Worker (works, no Rust) | **goose-owned `Goose.xcframework` — no Rust source in app** |
| Background drop | unavoidable | unavoidable (same) |
| Resume cost | full TCP+TLS+WS | **0-RTT QUIC** |
| Roaming | full reconnect | **QUIC migration (often seamless)** |
| Mid-stream resume | app-level cursor | app-level cursor (same) |
| Wake while asleep | needs push | needs push (same) |
| New infra | Cloudflare Worker | iroh relay server(s) (or n0 defaults) |

## 8. Roll our own FFI — don't ship `iroh-ffi`/`IrohLib`

`n0-computer/iroh-ffi` is explicitly a **reference/example**, not a product
dependency (`publish = false`; README says "for example"). Its surface is the
*whole* iroh toolkit — `src/{blob,doc,gossip,author,tag,node,net,endpoint,key,
ticket}.rs` (blobs, docs, gossip, authors…) — none of which we want on a mobile
ACP tunnel. We want a **tiny goose-owned crate** that exposes only: connect to a
paired goosed, open an ACP stream, push/pull ACP frames, resume cursor, cancel,
disconnect, observe path (direct/relayed). Pinning to their broad, fast-moving
binding (currently iroh `0.35`, UniFFI `0.28`) would drag in surface and version
churn we don't control.

**What we copy from `iroh-ffi` is the *recipe*, not the code:**
- Crate layout: `crate-type = ["staticlib", "cdylib"]`, `uniffi::setup_scaffolding!()`,
  a `uniffi-bindgen` bin, `lto = true`, deps = just `iroh` + `tokio` + our ACP types.
- Build pipeline (`make_swift.sh`): `cargo build --release` for
  `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios`,
  `aarch64-apple-darwin` → `uniffi-bindgen generate --language swift` →
  `lipo` the sim archs → assemble `Goose.xcframework` (ios-arm64 + sim + macos) →
  generate the Swift interface → `swift package compute-checksum` for a binary
  Swift Package target. ~80 lines of shell we adapt verbatim.

**Proposed `crates/goose-tunnel-ffi/`** (or live alongside the existing client SDK):
```
goose-tunnel-ffi/
  Cargo.toml            # iroh, tokio, uniffi, goose ACP types only
  src/lib.rs            # uniffi::setup_scaffolding!()
  src/tunnel.rs         # #[uniffi::export] GooseTunnel: connect/stream/cancel/status
  uniffi-bindgen.rs
  make_swift.sh         # adapted from iroh-ffi
  GooseTunnel.xcframework (generated artifact, or released as a zip)
```
Exported surface (UniFFI), goose-shaped, ALPN `goose-acp/1`, relays baked in:
```
generate_device_keypair() -> String
connect(server_addr_token, device_keypair, resume_after) -> GooseTunnel   // async, 0-RTT capable
GooseTunnel.send_acp(json)
GooseTunnel.stream(req, listener: AcpEventListener)   // callback iface, replaces SSE
GooseTunnel.cancel(request_id)
GooseTunnel.path_kind() -> Direct | Relayed           // from remote_info_list/home_relay
GooseTunnel.disconnect()
```
The Swift app imports this single framework — no Rust source, no toolchain, and a
surface we own and version with goose, not with n0's example repo.

> Server side reuses the same `iroh` crate directly in goosed (no FFI there) — see §6.

### Where does the Rust live? (three viable placements)

The FFI crate is normal Rust; the only question is *which repo/build owns it*. The
Swift app always consumes a **binary `.xcframework`** regardless — so "Rust in the
Swift codebase" is really about whether the Rust *source* and its build sit in the
mobile repo, the goose monorepo, or standalone.

1. **In the goose monorepo** (`crates/goose-tunnel-ffi/`) — *recommended default.*
   - Pros: one `iroh` version pinned across goosed + FFI (critical — §9.6); shares
     ACP types with `crates/goose/src/acp`; CI builds the xcframework as a release
     artifact; client/server protocol can't drift.
   - Cons: mobile release cadence coupled to monorepo; iOS toolchain in goose CI.
   - The Swift app references the published `Goose.xcframework` (versioned release
     zip + checksum), exactly how mesh-llm's `Package.swift` pulls a remote
     `MeshLLMFFI.xcframework.zip`.

2. **In the Swift app repo** (Rust crate as a subdirectory, built by an Xcode
   build phase / `make_swift.sh`).
   - Pros: mobile team owns cadence; Rust+Swift co-located, one PR changes both.
   - Cons: must vendor/track the right `iroh` + goose ACP types out-of-tree; easy
     for server and client iroh versions to drift (the §9.6 risk); duplicates the
     Rust toolchain into the mobile repo. This is the literal "Rust in the Swift
     codebase" option — it *works* (mesh-llm proves the build), but the version-skew
     risk makes it weaker than (1) for a protocol shared with goosed.

3. **Its own repo/crate** (`goose-tunnel` published crate + released xcframework).
   - Pros: clean dependency for both goosed and the app; independent versioning;
     reusable by a future Android/Kotlin client (UniFFI already does Kotlin).
   - Cons: a third repo to release-coordinate; still needs the iroh-version pin
     enforced across three consumers.

**Recommendation:** start with (1) — crate in the monorepo, xcframework as a CI
release artifact the Swift app consumes by version. It eliminates the protocol/iroh
drift risk by construction. Promote to (3) only if/when a second client (Android)
or external consumers appear. (2) is fine for a fast prototype but I'd avoid it as
the long-term home because the client and server share the iroh wire protocol and
must move together.

## 9. Open questions / next steps

1. **ACP as a first-class network transport** — generalize the stdio ACP server
   over a stream, or bridge-to-HTTP first for a faster cutover?
2. Own relays vs. n0 default vs. configurable (likely all three).
3. Enrollment UX: plain NodeId allow-list vs. signed ownership certs (mesh-llm style).
4. Migration: ship iroh transport behind a flag alongside lapstone, then delete.
5. APNs design for asleep-delivery (relay-side vs. goosed-side hook).
6. Pin a single `iroh` version across goosed + `goose-tunnel-ffi` (server and FFI
   must speak the same relay/disco protocol — it changes between iroh versions).
7. Replace `secure_compare` now regardless (security fix to current code).
