@testable import FloatViewMac
import Testing

@Suite("macOS startup recovery")
struct MacStartupStageTests {
  @Test("connection failures explain how to resolve competing app instances")
  func connectionRecovery() {
    let stage = MacStartupStage.connection
    #expect(stage.title.contains("local connection"))
    #expect(stage.recovery.contains("Quit any other copies, including test builds"))
    #expect(stage.recovery.contains("Try again"))
    #expect(!stage.recovery.contains("data"))
    #expect(!stage.recovery.contains("compatible version"))
  }

  @Test("data failures retain compatibility and backup guidance")
  func dataRecovery() {
    let stage = MacStartupStage.data
    #expect(stage.title.contains("data"))
    #expect(stage.recovery.contains("compatible version"))
    #expect(stage.recovery.contains("available storage"))
    #expect(stage.recovery.contains("backup"))
    #expect(!stage.recovery.contains("local connection"))
  }
}
