import FloatViewBackend
@testable import FloatViewMac
import Foundation
import Testing

@Suite("macOS launch configuration")
struct MacLaunchConfigurationTests {
  @Test("production cannot redirect its UI or storage")
  func overrides() {
    let configuration = MacLaunchConfiguration(environment: [
      "FLOAT_VIEW_FRONTEND_HTML": "/tmp/untrusted.html",
      "FLOAT_VIEW_LOG_ROOT": "/tmp/untrusted-storage",
    ])
    #if DEBUG
      #expect(configuration.frontendHTML?.path == "/tmp/untrusted.html")
      #expect(configuration.storageRoot?.path == "/tmp/untrusted-storage")
    #else
      #expect(configuration.frontendHTML == nil)
      #expect(configuration.storageRoot == nil)
    #endif
  }

  @Test("ordinary launch uses the application-owned defaults")
  func defaults() {
    let configuration = MacLaunchConfiguration(environment: [:])
    #expect(configuration.frontendHTML == nil)
    #expect(configuration.storageRoot == nil)
  }
}
