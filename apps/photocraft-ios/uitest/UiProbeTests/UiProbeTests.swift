import Foundation
import XCTest

// A scripted UI probe for the simulator: plays a JSON list of steps (the file named by
// PROBE_SCRIPT) against the app with bundle id PROBE_BUNDLE, writing screenshots and
// accessibility trees under PROBE_OUT. It exists because the agent has no macOS accessibility
// permission to click in Simulator.app: XCUITest reaches the app from inside the simulator.
// Steps are exploratory tools, not assertions; a step that cannot be done is logged to
// `steps.log` and the run goes on. See README.md.

struct Reveal: Decodable {
    var label: String
    var swipe: [Double]
}

struct Step: Decodable {
    var launch: [String: String]? // terminate + launch with these environment variables (ignored in attach mode)
    var tap: [Double]? // x, y: a point of the display (a screenshot's pixels over 2)
    var type: String? // text typed into whatever has keyboard focus
    var swipe: [Double]? // x1, y1, x2, y2, hold (seconds) at the end
    var key: String? // a key of the hardware keyboard, by name ("return", "escape")
    var sleep: Double? // seconds
    var shot: String? // screenshot file name, without the prefix or `.png`
    var tree: String? // accessibility tree dump file name, without the prefix or `.txt`
    var label: String? // tap the first element whose accessibility label is exactly this
    var waitLabel: String? // wait (up to 40 s) for an element whose label is exactly this
    var reveal: Reveal? // swipe again (up to 5 times) until the labelled element is on screen
}

final class UiProbeTests: XCTestCase {
    private var app: XCUIApplication!
    private var outDir = "/tmp/uiprobe-out"
    private var attach = false
    private var log = ""

    override func setUpWithError() throws {
        continueAfterFailure = true
        let env = ProcessInfo.processInfo.environment
        outDir = env["PROBE_OUT"] ?? outDir
        try FileManager.default.createDirectory(atPath: outDir, withIntermediateDirectories: true)
        attach = env["PROBE_ATTACH"] == "1"
        app = XCUIApplication(bundleIdentifier: env["PROBE_BUNDLE"] ?? "photoComfy")
        app.launchEnvironment = (env["PROBE_LAUNCH_ENV"] ?? "").split(separator: ",").reduce(into: [:]) { dict, pair in
            let kv = pair.split(separator: "=", maxSplits: 1).map(String.init)
            if kv.count == 2 { dict[kv[0]] = kv[1] }
        }
    }

    private func write(_ data: Data, _ name: String) {
        try? data.write(to: URL(fileURLWithPath: "\(outDir)/\(name)"))
    }

    private var recordedFrames = false
    private func point(_ p: [Double]) -> XCUICoordinate {
        // XCUITest taps land in the app's logical space, which an iPad simulator may report larger
        // than its display (1042x1512 for 834x1210 points): a tap passed as a display point then
        // lands at 0.8x of it. So a display point is passed PROBE_TAP_SCALE times itself, against
        // PROBE_SCREEN, the display's size in points.
        if !recordedFrames {
            recordedFrames = true
            let windows = app.windows.allElementsBoundByIndex.map { "\($0.frame) \($0.label)" }.joined(separator: "; ")
            write("app.frame=\(app.frame) windows=[\(windows)]\n".data(using: .utf8)!, "frames.txt")
        }
        let env = ProcessInfo.processInfo.environment
        let parts = (env["PROBE_SCREEN"] ?? "834x1210").split(separator: "x")
        let size = CGSize(width: Double(parts.first ?? "834") ?? 834, height: Double(parts.count > 1 ? parts[1] : "1210") ?? 1210)
        let scale = Double(env["PROBE_TAP_SCALE"] ?? "1.25") ?? 1.25
        return app.coordinate(withNormalizedOffset: CGVector(dx: p[0] * scale / size.width, dy: p[1] * scale / size.height))
    }

    private func labelled(_ label: String) -> XCUIElementQuery {
        app.descendants(matching: .any).matching(NSPredicate(format: "label == %@", label))
    }

    func testProbe() throws {
        let env = ProcessInfo.processInfo.environment
        let steps = try JSONDecoder().decode([Step].self, from: Data(contentsOf: URL(fileURLWithPath: env["PROBE_SCRIPT"] ?? "/tmp/uiprobe.json")))
        if attach {
            app.activate()
            sleep(4)
        } else {
            app.launch()
            sleep(6)
        }
        for (index, step) in steps.enumerated() {
            let tag = String(format: "%02d", index)
            func attempt(_ what: String, _ body: () throws -> Void) {
                do {
                    try body()
                } catch {
                    log += "\(tag) \(what): \(error)\n"
                    write(log.data(using: .utf8)!, "steps.log")
                }
            }
            if let environment = step.launch {
                if !attach {
                    app.terminate()
                    app.launchEnvironment = environment
                    app.launch()
                    sleep(6)
                }
            }
            if let p = step.tap {
                attempt("tap \(p)") { self.point(p).tap() }
            }
            if let l = step.label {
                attempt("label \(l)") {
                    // The accessibility tree fills in over the first frames after a launch, and
                    // only once the screen was touched: keep asking for the element.
                    var found = false
                    for _ in 0..<20 where !found {
                        if let first = self.labelled(l).allElementsBoundByIndex.first {
                            first.tap()
                            found = true
                        } else {
                            usleep(500_000)
                        }
                    }
                    if !found {
                        self.write("no element with label \(l)\n".data(using: .utf8)!, "\(tag)-missing-label.txt")
                    }
                }
            }
            if let w = step.waitLabel {
                for _ in 0..<80 where labelled(w).allElementsBoundByIndex.isEmpty { usleep(500_000) }
                if labelled(w).allElementsBoundByIndex.isEmpty {
                    write("timed out waiting for label \(w)\n".data(using: .utf8)!, "\(tag)-wait-timeout.txt")
                }
            }
            if let r = step.reveal, r.swipe.count >= 4 {
                let width = app.frame.width
                for _ in 0..<5 {
                    let onScreen = labelled(r.label).allElementsBoundByIndex.contains { $0.exists && $0.frame.minX >= 0 && $0.frame.maxX < width }
                    if onScreen { break }
                    point(Array(r.swipe[0..<2])).press(forDuration: 0.05, thenDragTo: point(Array(r.swipe[2..<4])), withVelocity: .slow, thenHoldForDuration: 0.1)
                    usleep(800_000)
                }
            }
            if let k = step.key {
                attempt("key \(k)") { self.app.typeKey(k, modifierFlags: []) }
            }
            if let t = step.type {
                attempt("type \(t)") { self.app.typeText(t) }
            }
            if let s = step.swipe, s.count >= 4 {
                attempt("swipe \(s)") {
                    self.point(Array(s[0..<2])).press(forDuration: 0.05, thenDragTo: self.point(Array(s[2..<4])), withVelocity: .slow, thenHoldForDuration: s.count > 4 ? s[4] : 0)
                }
            }
            if let t = step.tree {
                write(app.debugDescription.data(using: .utf8)!, "\(tag)-\(t).txt")
            }
            if let s = step.sleep {
                usleep(UInt32(max(0, s) * 1_000_000))
            }
            if let name = step.shot {
                write(XCUIScreen.main.screenshot().pngRepresentation, "\(tag)-\(name).png")
            }
        }
        write(log.data(using: .utf8)!, "steps.log")
        if !attach { app.terminate() }
    }
}
