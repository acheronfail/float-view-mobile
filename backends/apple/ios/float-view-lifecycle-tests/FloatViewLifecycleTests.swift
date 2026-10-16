import XCTest

final class FloatViewLifecycleTests: XCTestCase {
  private let application = XCUIApplication(bundleIdentifier: "fail.acheron.float-view")

  private func acceptSafetyIfNeeded() {
    let agreement = application.buttons["I agree"]
    let ready = NSPredicate { [self] _, _ in
      agreement.exists || application.links["settings"].isHittable
    }
    expectation(for: ready, evaluatedWith: nil)
    waitForExpectations(timeout: 15)
    if agreement.exists {
      agreement.tap()
    }
  }

  func testStartupAndForegroundPresentation() {
    for _ in 0 ..< 3 {
      application.terminate()
      application.launch()
      acceptSafetyIfNeeded()
      XCTAssertTrue(
        application.links["settings"].waitForExistence(timeout: 15),
        "Startup did not expose frontend navigation"
      )
      XCTAssertFalse(application.staticTexts["Unable to start Float View"].exists)
      XCUIDevice.shared.press(.home)
      application.activate()
      XCTAssertTrue(
        application.links["settings"].waitForExistence(timeout: 15),
        "Foreground reconstruction did not expose navigation"
      )
      XCTAssertFalse(application.staticTexts["Unable to start Float View"].exists)
    }
  }

  func testPermissionsRecovery() {
    continueAfterFailure = false
    application.terminate()
    application.resetAuthorizationStatus(for: .location)
    application.launch()
    acceptSafetyIfNeeded()
    XCTAssertTrue(application.links["settings"].waitForExistence(timeout: 15))
    application.links["settings"].tap()
    let request = application.buttons["Request access"].firstMatch
    if !request.isHittable {
      application.swipeUp()
    }
    if !request.waitForExistence(timeout: 3) {
      // Some physical iOS versions retain a prior denied choice through XCTest's reset.
      let open = application.buttons["Open settings"].firstMatch
      XCTAssertTrue(open.waitForExistence(timeout: 10))
      open.tap()
      let settings = XCUIApplication(bundleIdentifier: "com.apple.Preferences")
      XCTAssertTrue(settings.wait(for: .runningForeground, timeout: 10))
      let location = settings.cells.containing(.staticText, identifier: "Location").firstMatch
      XCTAssertTrue(location.waitForExistence(timeout: 10), settings.debugDescription)
      location.tap()
      let ask = settings.cells.matching(NSPredicate(format: "label CONTAINS[c] %@", "Ask Next Time")).firstMatch
      XCTAssertTrue(ask.waitForExistence(timeout: 10), settings.debugDescription)
      ask.tap()
      application.activate()
      XCTAssertTrue(application.links["settings"].waitForExistence(timeout: 15))
      application.links["settings"].tap()
      if !request.isHittable {
        application.swipeUp()
      }
    }
    XCTAssertTrue(request.waitForExistence(timeout: 10), application.debugDescription)
    request.tap()
    let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
    let deny = springboard.buttons.matching(NSPredicate(format: "label MATCHES %@", "Don.t Allow")).firstMatch
    XCTAssertTrue(deny.waitForExistence(timeout: 10), springboard.debugDescription)
    deny.tap()
    let open = application.buttons["Open settings"].firstMatch
    XCTAssertTrue(open.waitForExistence(timeout: 10), application.debugDescription)
    open.tap()
    let settings = XCUIApplication(bundleIdentifier: "com.apple.Preferences")
    XCTAssertTrue(settings.wait(for: .runningForeground, timeout: 10))
    application.activate()
    XCTAssertTrue(application.links["settings"].waitForExistence(timeout: 15))
    application.links["settings"].tap()
    if !open.isHittable {
      application.swipeUp()
    }
    XCTAssertTrue(open.waitForExistence(timeout: 10), "Returning without change must retain denied recovery")
    open.tap()
    XCTAssertTrue(settings.wait(for: .runningForeground, timeout: 10))
    let location = settings.cells.containing(.staticText, identifier: "Location").firstMatch
    XCTAssertTrue(location.waitForExistence(timeout: 10), settings.debugDescription)
    location.tap()
    let always = settings.cells.containing(.staticText, identifier: "Always").firstMatch
    XCTAssertTrue(always.waitForExistence(timeout: 10), settings.debugDescription)
    always.tap()
    application.activate()
    XCTAssertTrue(application.links["settings"].waitForExistence(timeout: 15))
    application.links["settings"].tap()
    if !application.staticTexts["Always"].isHittable {
      application.swipeUp()
    }
    XCTAssertTrue(application.staticTexts["Always"].waitForExistence(timeout: 10), application.debugDescription)
    XCTAssertFalse(application.buttons["Request access"].exists)
  }

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
