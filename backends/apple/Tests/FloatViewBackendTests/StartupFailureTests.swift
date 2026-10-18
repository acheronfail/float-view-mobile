import FloatViewBackend
import Testing

@Suite("Native startup recovery")
struct StartupFailureTests {
  @Test("mobile data recovery protects sandboxed data without asking for a manual backup")
  func mobileDataRecovery() {
    let text = StartupFailure.data.recovery(on: .iOS)
    #expect(text.contains("Settings"))
    #expect(text.contains("Do not delete the app"))
    #expect(!text.contains("backup"))
    #expect(!text.contains("other copies"))
  }

  @Test("connection recovery is specific to each platform")
  func connectionRecovery() {
    #expect(StartupFailure.connection.title.contains("local connection"))
    #expect(StartupFailure.connection.recovery(on: .macOS).contains("Quit any other copies"))
    #expect(StartupFailure.connection.recovery(on: .iOS).contains("restart your iPhone or iPad"))
    #expect(!StartupFailure.connection.recovery(on: .iOS).contains("data"))
  }

  @Test("data and interface failures have distinct recovery")
  func distinctFailures() {
    #expect(StartupFailure.data.title.contains("data"))
    #expect(StartupFailure.data.recovery(on: .macOS).contains("backup"))
    #expect(StartupFailure.interface.title == "Unable to load Float View")
    #expect(StartupFailure.interface.recovery(on: .iOS) == StartupFailure.interface.recovery(on: .macOS))
  }
}
