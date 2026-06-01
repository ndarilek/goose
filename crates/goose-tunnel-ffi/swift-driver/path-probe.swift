import Foundation

// Path probe: connect with a relay-only token (the off-network mobile case) and
// sample path_kind() over time to observe iroh's relay -> direct upgrade.
//
// On a single host a loopback direct path always exists, so iroh correctly
// upgrades relay -> direct after the disco handshake. This probe documents that
// progression honestly rather than pretending the relay path is permanent.

final class Probe: MessageListener {
    let sema = DispatchSemaphore(value: 0)
    var gotInit = false
    func onMessage(line: String) {
        if let d = line.data(using: .utf8),
           let o = try? JSONSerialization.jsonObject(with: d) as? [String: Any],
           o["result"] != nil, (o["id"] as? Int) == 1 { gotInit = true; sema.signal() }
    }
    func onClosed(reason: String) {}
}

@main
struct PathProbe {
    static func main() {
        guard CommandLine.arguments.count >= 2 else {
            FileHandle.standardError.write("usage: path-probe <server-token>\n".data(using: .utf8)!)
            exit(2)
        }
        let token = CommandLine.arguments[1]
        let deviceKey = generateDeviceKeypair()
        let probe = Probe()
        do {
            let tunnel = try connect(serverToken: token, deviceKeyHex: deviceKey, listener: probe)
            print("t=0.0s  path: \(tunnel.pathKind())   (just connected)")
            let initReq: [String: Any] = ["jsonrpc": "2.0", "id": 1, "method": "initialize",
                                          "params": ["protocolVersion": 1]]
            try tunnel.send(line: String(data: try JSONSerialization.data(withJSONObject: initReq), encoding: .utf8)!)
            _ = probe.sema.wait(timeout: .now() + 20)
            print("        ACP initialize: \(probe.gotInit ? "ok" : "FAILED")")
            for t in [1.0, 2.0, 4.0] {
                Thread.sleep(forTimeInterval: t == 1.0 ? 1.0 : (t == 2.0 ? 1.0 : 2.0))
                print("t=\(t)s  path: \(tunnel.pathKind())")
            }
            tunnel.disconnect()
            exit(probe.gotInit ? 0 : 1)
        } catch {
            print("❌ \(error)"); exit(1)
        }
    }
}
