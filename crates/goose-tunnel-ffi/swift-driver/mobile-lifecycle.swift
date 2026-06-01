import Foundation

// Mobile lifecycle harness: exercises the iOS suspend/resume cycle against a real
// goosed iroh server, the way the mobile app would actually behave.
//
//   foreground -> connect -> ACP initialize
//   background -> disconnect (iOS freezes the process; UDP socket dies)
//   foreground -> reconnect (fresh tunnel) -> ACP initialize again  [x3]
//
// This is the part a plain CLI run does NOT cover: connection teardown on
// suspend and fast re-establishment on resume, repeatedly, over real QUIC/relay.

final class CycleListener: MessageListener {
    private let sema: DispatchSemaphore
    private(set) var gotInit = false
    init(_ s: DispatchSemaphore) { sema = s }
    func onMessage(line: String) {
        if let d = line.data(using: .utf8),
           let o = try? JSONSerialization.jsonObject(with: d) as? [String: Any],
           o["result"] != nil, (o["id"] as? Int) == 1 {
            gotInit = true
            sema.signal()
        }
    }
    func onClosed(reason: String) { /* expected on background teardown */ }
}

@main
struct MobileLifecycle {
    static func foregroundConnectAndInit(token: String, deviceKey: String) -> (ok: Bool, path: String, ms: Int) {
        let sema = DispatchSemaphore(value: 0)
        let listener = CycleListener(sema)
        let start = Date()
        do {
            let tunnel = try connect(serverToken: token, deviceKeyHex: deviceKey, listener: listener)
            let initReq: [String: Any] = ["jsonrpc": "2.0", "id": 1, "method": "initialize",
                                          "params": ["protocolVersion": 1]]
            let line = String(data: try JSONSerialization.data(withJSONObject: initReq), encoding: .utf8)!
            try tunnel.send(line: line)
            let waited = sema.wait(timeout: .now() + 20)
            let ms = Int(Date().timeIntervalSince(start) * 1000)
            let path = "\(tunnel.pathKind())"
            // Simulate iOS backgrounding: clean-close so we don't leave a zombie.
            tunnel.disconnect()
            return (waited == .success && listener.gotInit, path, ms)
        } catch {
            return (false, "error: \(error)", Int(Date().timeIntervalSince(start) * 1000))
        }
    }

    static func main() {
        guard CommandLine.arguments.count >= 2 else {
            FileHandle.standardError.write("usage: mobile-lifecycle <server-token>\n".data(using: .utf8)!)
            exit(2)
        }
        let token = CommandLine.arguments[1]

        // The device identity is generated once and persisted (Keychain on a real
        // phone). Reused across every foreground/reconnect so the server sees a
        // stable NodeId.
        let deviceKey = generateDeviceKeypair()
        print("📱 device identity (stable across suspend/resume): \(deviceKey.prefix(16))…")

        var allOk = true
        let cycles = 3
        for cycle in 1...cycles {
            print("\n🔆 FOREGROUND #\(cycle): connecting over iroh…")
            let r = foregroundConnectAndInit(token: token, deviceKey: deviceKey)
            if r.ok {
                print("   ✅ ACP initialize ok  | path: \(r.path) | \(r.ms)ms")
            } else {
                print("   ❌ failed | \(r.path) | \(r.ms)ms")
                allOk = false
            }
            if cycle < cycles {
                print("🌙 BACKGROUND: app suspended, tunnel torn down (UDP frozen on real iOS)")
                Thread.sleep(forTimeInterval: 2.0)
            }
        }

        print("\n\(allOk ? "✅ PASS" : "❌ FAIL"): survived \(cycles) suspend/resume cycles with stable device identity")
        exit(allOk ? 0 : 1)
    }
}
