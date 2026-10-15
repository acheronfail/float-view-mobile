@testable import FloatViewBackend
import Foundation
import XCTest

final class LiveActivityUpdatePolicyTests: XCTestCase {
  private let start = Date(timeIntervalSince1970: 1800000000)

  private func presentation(
    load: String = "42%", source: String? = "duty", voltage: String = "75.6 V",
    temperature: String = "54°C", temperatureSource: String? = "Motor", temperatureRGB: [Int]? = [226, 58, 28],
    status: String? = nil, left: String = "active", right: String = "active"
  ) -> LiveActivityUpdatePolicy.Presentation {
    .init(source: source, load: load, distance: "1.23 km", battery: "67%", voltage: voltage,
          temperature: temperature, temperatureSource: temperatureSource, temperatureRGB: temperatureRGB,
          status: status, footpadLeft: left, footpadRight: right, imperial: false)
  }

  private func startedPolicy() -> LiveActivityUpdatePolicy {
    var policy = LiveActivityUpdatePolicy()
    policy.submitted(presentation(), observedAt: start, now: start, uptime: 100)
    return policy
  }

  func testFirstSubmissionIsImmediate() {
    let policy = LiveActivityUpdatePolicy()
    XCTAssertEqual(policy.reason(for: presentation(), observedAt: start, now: start, uptime: 100), .initial)
  }

  func testTimestampOnlyChangesRefreshAtTenSeconds() {
    let policy = startedPolicy()
    for elapsed in [1.0, 5, 9.999] {
      let now = start.addingTimeInterval(elapsed)
      XCTAssertNil(policy.reason(for: presentation(), observedAt: now, now: now, uptime: 100 + elapsed))
    }
    let now = start.addingTimeInterval(10)
    XCTAssertEqual(policy.reason(for: presentation(), observedAt: now, now: now, uptime: 110), .freshness)
  }

  func testNumericAndColourChangesWaitFiveSecondsAndCoalesce() {
    var policy = startedPolicy()
    let changed = presentation(load: "43%", source: "motor", voltage: "75.5 V")
    let now = start.addingTimeInterval(4.999)
    XCTAssertNil(policy.reason(for: changed, observedAt: now, now: now, uptime: 104.999))
    let next = start.addingTimeInterval(5)
    XCTAssertEqual(policy.reason(for: changed, observedAt: next, now: next, uptime: 105), .display)
    policy.submitted(changed, observedAt: next, now: next, uptime: 105)
    XCTAssertNil(policy.reason(for: presentation(), observedAt: next, now: next, uptime: 106))
  }

  func testTemperatureAndSensorChangesUseDisplayCadence() {
    let policy = startedPolicy()
    for changed in [
      presentation(temperature: "55°C"),
      presentation(temperatureSource: "ESC"), presentation(temperatureRGB: [227, 59, 27]),
      presentation(temperature: "—", temperatureSource: nil),
    ] {
      XCTAssertNil(policy.reason(for: changed, observedAt: start, now: start, uptime: 104))
      XCTAssertEqual(policy.reason(for: changed, observedAt: start, now: start, uptime: 105), .display)
    }
  }

  func testTransientNumericChangeThatReturnsToDisplayedValueIsNotSent() {
    let policy = startedPolicy()
    XCTAssertNil(policy.reason(for: presentation(load: "43%"), observedAt: start, now: start, uptime: 101))
    XCTAssertNil(policy.reason(for: presentation(), observedAt: start, now: start, uptime: 105))
  }

  func testStatusAndBothFootpadTransitionsBypassNumericCadence() {
    for changed in [presentation(status: "Stopped"), presentation(left: "warning"), presentation(right: "neutral")] {
      let policy = startedPolicy()
      XCTAssertEqual(policy.reason(for: changed, observedAt: start, now: start, uptime: 101), .transition)
    }
  }

  func testStaleSourceDoesNotReceiveFreshnessHeartbeatsAndRecoveryIsImmediate() {
    var policy = startedPolicy()
    XCTAssertNil(policy.reason(for: presentation(), observedAt: start, now: start.addingTimeInterval(10), uptime: 110))
    let stale = start.addingTimeInterval(15)
    XCTAssertEqual(policy.reason(for: presentation(), observedAt: start, now: stale, uptime: 115), .transition)
    policy.submitted(presentation(), observedAt: start, now: stale, uptime: 115)
    XCTAssertNil(policy.reason(for: presentation(), observedAt: start, now: stale.addingTimeInterval(20), uptime: 135))
    let recovered = stale.addingTimeInterval(1)
    XCTAssertEqual(policy.reason(for: presentation(), observedAt: recovered, now: recovered, uptime: 116), .transition)
  }

  func testMissingObservationIsStaleAndDoesNotRefreshRepeatedly() {
    var policy = LiveActivityUpdatePolicy()
    policy.submitted(presentation(), observedAt: nil, now: start, uptime: 100)
    XCTAssertNil(policy.reason(for: presentation(), observedAt: nil, now: start, uptime: 120))
    XCTAssertEqual(policy.reason(for: presentation(), observedAt: start, now: start, uptime: 101), .transition)
  }

  func testFreshObservationReplacesExpiredPayloadWithoutWaitingForHeartbeat() {
    var policy = LiveActivityUpdatePolicy()
    policy.submitted(presentation(), observedAt: start.addingTimeInterval(-14), now: start, uptime: 100)
    let now = start.addingTimeInterval(1)
    XCTAssertEqual(policy.reason(for: presentation(), observedAt: now, now: now, uptime: 101), .transition)
  }

  func testCadenceUsesMonotonicTimeDespiteWallClockChange() {
    let policy = startedPolicy()
    let adjusted = start.addingTimeInterval(-3600)
    XCTAssertNil(policy.reason(for: presentation(load: "44%"), observedAt: adjusted, now: adjusted, uptime: 104))
    XCTAssertEqual(
      policy.reason(for: presentation(load: "44%"), observedAt: adjusted, now: adjusted, uptime: 105),
      .display
    )
  }
}
