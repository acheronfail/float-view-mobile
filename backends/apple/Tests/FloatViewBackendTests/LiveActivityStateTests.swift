@testable import FloatViewBackend
import Foundation
import XCTest

final class LiveActivityStateTests: XCTestCase {
  func testNativeProjectionDecodesWithoutFrontend() throws {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let data = Data(#"{"generation":7,"observed_at":1800000000000,"source":"motor","percentage":82,"distance_meters":1234,"battery_percent":67,"imperial":true}"#
      .utf8)
    let state = try decoder.decode(LiveActivityState.self, from: data)
    XCTAssertEqual(state.generation, 7)
    XCTAssertEqual(state.observedAt, 1800000000000)
    XCTAssertEqual(state.source, "motor")
    XCTAssertEqual(state.percentage, 82)
    XCTAssertEqual(state.distanceMeters, 1234)
    XCTAssertEqual(state.batteryPercent, 67)
    XCTAssertTrue(state.imperial)
  }
}
