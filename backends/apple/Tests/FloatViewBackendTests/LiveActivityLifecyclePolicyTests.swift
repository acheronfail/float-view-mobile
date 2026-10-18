@testable import FloatViewBackend
import XCTest

final class LiveActivityLifecyclePolicyTests: XCTestCase {
  func testReadyConnectionCanStartWithoutRidingButRequiresForegroundAndAuthorization() {
    var policy = LiveActivityLifecyclePolicy()
    XCTAssertFalse(policy.canStart(foreground: true, enabled: true, uptime: 100))
    policy.receive(generation: 7)
    XCTAssertTrue(policy.canStart(foreground: true, enabled: true, uptime: 100))
    XCTAssertFalse(policy.canStart(foreground: false, enabled: true, uptime: 100))
    XCTAssertFalse(policy.canStart(foreground: true, enabled: false, uptime: 100))
  }

  func testDismissalSurvivesReconnectAndForegroundUntilExplicitShow() {
    var policy = LiveActivityLifecyclePolicy()
    policy.receive(generation: 7)
    policy.activityFinished(.dismissed)
    for _ in 0 ..< 3 {
      policy.receive(generation: 7)
      XCTAssertFalse(policy.canStart(foreground: true, enabled: true, uptime: 100))
    }
    policy.show()
    XCTAssertFalse(policy.canStart(foreground: false, enabled: true, uptime: 100))
    XCTAssertTrue(policy.canStart(foreground: true, enabled: true, uptime: 100))
  }

  func testSystemExpiryCanRecoverAtNextForegroundOpportunity() {
    var policy = LiveActivityLifecyclePolicy()
    policy.receive(generation: 7)
    policy.activityFinished(.ended)
    XCTAssertFalse(policy.canStart(foreground: false, enabled: true, uptime: 100))
    XCTAssertTrue(policy.canStart(foreground: true, enabled: true, uptime: 100))
  }

  func testNewConnectionAndDisconnectClearDismissalAndRetryDelay() {
    var policy = LiveActivityLifecyclePolicy()
    policy.receive(generation: 7)
    policy.activityFinished(.dismissed)
    policy.startFailed(uptime: 100)
    policy.receive(generation: nil)
    XCTAssertFalse(policy.canStart(foreground: true, enabled: true, uptime: 101))
    policy.receive(generation: 8)
    XCTAssertTrue(policy.canStart(foreground: true, enabled: true, uptime: 101))
  }

  func testRepeatedTelemetryDoesNotBypassMonotonicRetryDelay() {
    var policy = LiveActivityLifecyclePolicy()
    policy.receive(generation: 7)
    policy.startFailed(uptime: 100)
    policy.receive(generation: 7)
    XCTAssertFalse(policy.canStart(foreground: true, enabled: true, uptime: 129.999))
    XCTAssertTrue(policy.canStart(foreground: true, enabled: true, uptime: 130))
    policy.show()
    XCTAssertTrue(policy.canStart(foreground: true, enabled: true, uptime: 101))
  }
}
