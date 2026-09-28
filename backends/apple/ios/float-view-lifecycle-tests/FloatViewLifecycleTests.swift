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

  func testFilePresentation() {
    application.activate()
    let close = application.buttons["Close"]
    XCTAssertTrue(close.waitForExistence(timeout: 10), "Native export sheet did not appear")
    close.tap()
    XCTAssertTrue(close.waitForNonExistence(timeout: 5), "Native export sheet did not dismiss")
  }
}
