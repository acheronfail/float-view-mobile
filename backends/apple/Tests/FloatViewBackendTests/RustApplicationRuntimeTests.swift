@testable import FloatViewBackend
import Foundation
import Testing

@Suite(.serialized)
struct RustApplicationRuntimeTests {
  @Test func coarseRustRuntimePublishesOneOrderedResult() throws {
    let runtime = try RustApplicationRuntime()
    let result = try runtime.apply([
      "command": "begin",
      "now_ms": 10,
      "idle_interval_ms": 1_000,
      "post_ride_tail_ms": 5_000,
      "preferred_controller_can_id": NSNull(),
      "preferred_bms_can_id": NSNull(),
      "routing_hint_can_id": NSNull(),
      "routing_hint_controller_uuid": NSNull(),
      "has_active_session": false,
      "continuity_can_id": NSNull(),
      "continuity_controller_uuid": NSNull(),
    ])

    #expect(result.generation == 1)
    #expect(result.effects.first?["effect"] as? String == "publish_state")
    #expect(result.effects.contains { $0["effect"] as? String == "write_bytes" })
    #expect(result.effects.contains { $0["effect"] as? String == "arm_deadline" })
    #expect(result.effects.last?["effect"] as? String == "publish_state")
  }

  @Test func coarseRustRuntimeRejectsOpenAndOversizedCommands() throws {
    let runtime = try RustApplicationRuntime()
    #expect(throws: RustApplicationRuntimeError.invalidCommand) {
      try runtime.apply(["command": "reset", "now_ms": 1, "extra": true])
    }
    #expect(throws: RustApplicationRuntimeError.invalidCommand) {
      try runtime.apply([
        "command": "reset",
        "now_ms": 1,
        "padding": String(repeating: "x", count: RustApplicationRuntime.maximumCommandBytes),
      ])
    }
  }
}
