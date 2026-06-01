import Foundation

// End-to-end Swift driver: connect to goosed over iroh, speak ACP (newline JSON-RPC).
// Proves: pure Swift -> goose-owned FFI -> iroh QUIC (relay/direct) -> goosed ACP.
//
// Build/run via crates/goose-tunnel-ffi/swift-driver/run.sh

final class ACPListener: MessageListener {
    let sema: DispatchSemaphore
    var sawInitialize = false
    init(_ sema: DispatchSemaphore) { self.sema = sema }

    func onMessage(line: String) {
        print("⬅️  \(line)")
        if let data = line.data(using: .utf8),
           let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
           obj["result"] != nil, obj["id"] as? Int == 1 {
            sawInitialize = true
            print("✅ ACP initialize round-trip succeeded over iroh")
            sema.signal()
        }
    }

    func onClosed(reason: String) {
        print("🔌 stream closed: \(reason)")
        sema.signal()
    }
}

guard CommandLine.arguments.count >= 2 else {
    FileHandle.standardError.write("usage: driver <server-token>\n".data(using: .utf8)!)
    exit(2)
}
let token = CommandLine.arguments[1]

let deviceKey = generateDeviceKeypair()
print("🔑 device key (NodeId pinned on pairing): \(deviceKey.prefix(16))…")

let done = DispatchSemaphore(value: 0)
let listener = ACPListener(done)

do {
    print("🌐 connecting over iroh…")
    let tunnel = try connect(serverToken: token, deviceKeyHex: deviceKey, listener: listener)
    print("🛣️  path: \(tunnel.pathKind())")

    let initReq: [String: Any] = [
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": ["protocolVersion": 1],
    ]
    let data = try JSONSerialization.data(withJSONObject: initReq)
    let line = String(data: data, encoding: .utf8)!
    print("➡️  \(line)")
    try tunnel.send(line: line)

    let result = done.wait(timeout: .now() + 30)
    if result == .timedOut { print("⏱️  timed out waiting for ACP response") }
    print("🛣️  final path: \(tunnel.pathKind())")
    tunnel.disconnect()
    exit(listener.sawInitialize ? 0 : 1)
} catch {
    print("❌ \(error)")
    exit(1)
}
