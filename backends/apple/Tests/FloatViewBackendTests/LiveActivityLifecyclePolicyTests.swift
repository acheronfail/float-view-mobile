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

  func testBackgroundReconnectDeadlineIsBoundedAndRepeatedSnapshotsDoNotExtendIt() {
    var policy = LiveActivityLifecyclePolicy()
    policy.receive(generation: 7)
    policy.reconnectChanged(reconnecting: true, foreground: false, uptime: 100)
    XCTAssertEqual(policy.reconnectDeadline, 110)
    XCTAssertFalse(policy.shouldEndReconnect(uptime: 105))
    policy.reconnectChanged(reconnecting: true, foreground: false, uptime: 109)
    XCTAssertEqual(policy.reconnectDeadline, 110)
    XCTAssertFalse(policy.shouldEndReconnect(uptime: 109.999))
    XCTAssertTrue(policy.shouldEndReconnect(uptime: 110))
    XCTAssertTrue(policy.shouldEndReconnect(uptime: 600))
    XCTAssertFalse(policy.canStart(foreground: true, enabled: true, uptime: 600))
  }

  func testRecoveryCancelsCleanupButCannotRecreateActivityUntilForeground() {
    var policy = LiveActivityLifecyclePolicy()
    policy.receive(generation: 7)
    policy.reconnectChanged(reconnecting: true, foreground: false, uptime: 100)
    policy.reconnectChanged(reconnecting: false, foreground: false, uptime: 103)
    XCTAssertNil(policy.reconnectDeadline)
    XCTAssertFalse(policy.shouldEndReconnect(uptime: 105))
    XCTAssertFalse(policy.canStart(foreground: false, enabled: true, uptime: 105))
    XCTAssertTrue(policy.canStart(foreground: true, enabled: true, uptime: 105))
    // A later independent loss gets its own grace period.
    policy.reconnectChanged(reconnecting: true, foreground: false, uptime: 200)
    XCTAssertEqual(policy.reconnectDeadline, 210)
  }

  func testForegroundReconnectCancelsBackgroundDeadlineWithoutAllowingNewActivity() {
    var policy = LiveActivityLifecyclePolicy()
    policy.receive(generation: 7)
    policy.reconnectChanged(reconnecting: true, foreground: true, uptime: 100)
    XCTAssertNil(policy.reconnectDeadline)
    policy.reconnectChanged(reconnecting: true, foreground: false, uptime: 200)
    XCTAssertEqual(policy.reconnectDeadline, 210)
    policy.reconnectChanged(reconnecting: true, foreground: true, uptime: 202)
    XCTAssertNil(policy.reconnectDeadline)
    XCTAssertFalse(policy.canStart(foreground: true, enabled: true, uptime: 203))
    policy.reconnectChanged(reconnecting: false, foreground: true, uptime: 204)
    XCTAssertTrue(policy.canStart(foreground: true, enabled: true, uptime: 204))
  }

  func testReplacementAndDisconnectClearOldReconnectDeadline() {
    var policy = LiveActivityLifecyclePolicy()
    policy.receive(generation: 7)
    policy.reconnectChanged(reconnecting: true, foreground: false, uptime: 100)
    policy.receive(generation: 8)
    XCTAssertNil(policy.reconnectDeadline)
    XCTAssertTrue(policy.canStart(foreground: true, enabled: true, uptime: 101))
    policy.reconnectChanged(reconnecting: true, foreground: false, uptime: 102)
    policy.receive(generation: nil)
    XCTAssertNil(policy.reconnectDeadline)
    policy.reconnectChanged(reconnecting: true, foreground: false, uptime: 103)
    XCTAssertNil(policy.reconnectDeadline)
  }

  func testRecoveryDoesNotOverrideUserDismissal() {
    var policy = LiveActivityLifecyclePolicy()
    policy.receive(generation: 7)
    policy.reconnectChanged(reconnecting: true, foreground: false, uptime: 100)
    policy.activityFinished(.dismissed)
    policy.reconnectChanged(reconnecting: false, foreground: true, uptime: 102)
    XCTAssertFalse(policy.canStart(foreground: true, enabled: true, uptime: 102))
  }
}
