import XCTest

final class FloatViewLifecycleTests: XCTestCase {
  private let application = XCUIApplication(bundleIdentifier: "fail.acheron.float-view")

  func testBackground() {
    application.activate()
    XCUIDevice.shared.press(.home)
  }

  func testForeground() {
    application.activate()
  }
}
